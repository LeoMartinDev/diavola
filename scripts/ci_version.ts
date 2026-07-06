import { formatVersion } from "./version_calc.ts";

const TAURI_CONF = "src-tauri/tauri.conf.json";
const CARGO_TOML = "src-tauri/Cargo.toml";
const PACKAGE_JSON = "package.json";

const dryRun = Deno.args.includes("--dry-run");

async function readPackageVersion(): Promise<string> {
  const text = await Deno.readTextFile(PACKAGE_JSON);
  const pkg = JSON.parse(text);
  if (!pkg.version || typeof pkg.version !== "string") {
    throw new Error("package.json does not contain a valid version field");
  }
  return pkg.version;
}

async function patchJson(path: string, version: string): Promise<void> {
  const text = await Deno.readTextFile(path);
  const json = JSON.parse(text);
  json.version = version;
  await Deno.writeTextFile(path, JSON.stringify(json, null, 2) + "\n");
}

async function patchCargoVersion(path: string, version: string): Promise<void> {
  const text = await Deno.readTextFile(path);
  const regex = /^(\[package\][\s\S]*?\nversion\s*=\s*")([^"]*)(")/m;
  if (!regex.test(text)) {
    throw new Error(
      `ci_version: could not find [package] version to patch in ${path}`,
    );
  }
  const updated = text.replace(regex, `$1${version}$3`);
  await Deno.writeTextFile(path, updated);
}

async function main(): Promise<void> {
  const ciVersion = Deno.env.get("CI_VERSION");
  const version = ciVersion || await readPackageVersion();
  const info = formatVersion(version);

  if (dryRun) {
    console.log(`APP_VERSION=${info.appVersion}`);
    console.log(`RELEASE_VERSION=${info.releaseVersion}`);
    console.log(`RELEASE_TAG=${info.releaseTag}`);
    console.log(`(dry-run: no files patched, no GITHUB_ENV written)`);
    return;
  }

  await patchJson(PACKAGE_JSON, info.appVersion);
  await patchJson(TAURI_CONF, info.appVersion);
  await patchCargoVersion(CARGO_TOML, info.appVersion);

  const ghEnv = Deno.env.get("GITHUB_ENV");
  if (ghEnv) {
    const line = [
      `APP_VERSION=${info.appVersion}`,
      `RELEASE_VERSION=${info.releaseVersion}`,
      `RELEASE_TAG=${info.releaseTag}`,
      "",
    ].join("\n");
    await Deno.writeTextFile(ghEnv, line, { append: true });
  }

  console.log(`APP_VERSION=${info.appVersion}`);
  console.log(`RELEASE_VERSION=${info.releaseVersion}`);
  console.log(`RELEASE_TAG=${info.releaseTag}`);
}

main();
