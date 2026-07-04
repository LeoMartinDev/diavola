#!/usr/bin/env node

const { spawn } = require("child_process");
const {
  createWriteStream,
  mkdirSync,
  chmodSync,
  existsSync,
  rmSync,
} = require("fs");
const { join, dirname } = require("path");
const { createGunzip } = require("zlib");
const { pipeline } = require("stream");
const { promisify } = require("util");
const os = require("os");

const streamPipeline = promisify(pipeline);

const REPO = "LeoMartinDev/diavola";
const BIN_NAME = process.platform === "win32" ? "diavola.exe" : "diavola";

function getPlatform() {
  const platform = os.platform();
  const arch = os.arch();

  const archMap = { x64: "amd64", arm64: "arm64" };
  const resolvedArch = archMap[arch] || arch;

  switch (platform) {
    case "linux":
      return `linux-${resolvedArch}`;
    case "darwin":
      return `darwin-${resolvedArch}`;
    case "win32":
      return `windows-${resolvedArch}`;
    default:
      throw new Error(`Unsupported platform: ${platform} ${arch}`);
  }
}

function getAssetName(platform) {
  if (platform.startsWith("linux")) return `diavola-${platform}.gz`;
  if (platform.startsWith("darwin")) return `diavola-${platform}.tar.gz`;
  if (platform.startsWith("windows")) return `diavola-${platform}.exe`;
  throw new Error(`Unknown platform: ${platform}`);
}

function getBinaryPath(installDir, platform) {
  if (platform.startsWith("darwin")) {
    return join(installDir, "Diavola.app", "Contents", "MacOS", "diavola");
  }
  return join(installDir, BIN_NAME);
}

async function getLatestVersion() {
  const res = await fetch(
    `https://api.github.com/repos/${REPO}/releases/latest`,
    {
      headers: { "User-Agent": "diavola-npm-installer" },
    },
  );
  if (!res.ok) throw new Error(`Failed to fetch latest release: ${res.status}`);
  const data = await res.json();
  return { version: data.tag_name, assets: data.assets };
}

async function downloadAndRun() {
  const platform = getPlatform();
  const assetName = getAssetName(platform);
  const installDir = join(os.homedir(), ".diavola", "bin");
  const versionFile = join(installDir, ".version");

  let release;
  try {
    release = await getLatestVersion();
  } catch (e) {
    console.error(
      `Unable to find latest Diavola release. Is the repo ${REPO} public?\n${e.message}`,
    );
    process.exit(1);
  }

  const asset = release.assets.find((a) => a.name === assetName);
  if (!asset) {
    console.error(
      `No binary found for ${platform} in release ${release.version}. ` +
        `Expected asset: ${assetName}`,
    );
    process.exit(1);
  }

  const binPath = getBinaryPath(installDir, platform);

  if (existsSync(binPath) && existsSync(versionFile)) {
    const localVersion = require("fs").readFileSync(versionFile, "utf8").trim();
    if (localVersion === release.version) {
      runBinary(binPath, platform);
      return;
    }
  }

  console.log(`Downloading Diavola ${release.version} for ${platform}...`);

  if (existsSync(installDir)) rmSync(installDir, { recursive: true });
  mkdirSync(installDir, { recursive: true });
  const tmpFile = join(installDir, `${assetName}.download`);

  await downloadFile(asset.browser_download_url, tmpFile);

  if (platform.startsWith("linux")) {
    console.log("Extracting...");
    await gunzip(tmpFile, binPath);
    chmodSync(binPath, 0o755);
  } else if (platform.startsWith("darwin")) {
    console.log("Extracting...");
    await extractTarGz(tmpFile, installDir);
    rmSync(tmpFile);
  } else if (platform.startsWith("windows")) {
    require("fs").renameSync(tmpFile, binPath);
  }

  require("fs").writeFileSync(versionFile, release.version);

  runBinary(binPath, platform);
}

function runBinary(binPath, platform) {
  console.log("Launching Diavola...");
  const opts = { stdio: "inherit" };
  if (!platform.startsWith("windows")) {
    opts.detached = true;
  }
  spawn(binPath, process.argv.slice(2), opts).unref();
}

async function downloadFile(url, dest) {
  const res = await fetch(url, {
    redirect: "follow",
    headers: { "User-Agent": "diavola-npm-installer" },
  });
  if (!res.ok) throw new Error(`Download failed: ${res.status}`);
  const buffer = Buffer.from(await res.arrayBuffer());
  require("fs").writeFileSync(dest, buffer);
}

function gunzip(src, dest) {
  return streamPipeline(
    require("fs").createReadStream(src),
    createGunzip(),
    createWriteStream(dest),
  ).then(() => rmSync(src));
}

function extractTarGz(src, dest) {
  const { execSync } = require("child_process");
  execSync(`tar -xzf "${src}" -C "${dest}"`, { stdio: "inherit" });
}

downloadAndRun().catch((err) => {
  console.error("Failed to download/run Diavola:", err.message);
  process.exit(1);
});
