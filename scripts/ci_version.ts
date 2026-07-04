import { computeVersion } from "./version_calc.ts";

const FILES = {
  tauriConf: "src-tauri/tauri.conf.json",
  packageJson: "package.json",
  cargoToml: "src-tauri/Cargo.toml",
};

const dryRun = Deno.args.includes("--dry-run");

async function git(args: string[]): Promise<string> {
  const cmd = new Deno.Command("git", {
    args,
    stdout: "piped",
    stderr: "piped",
  });
  const { code, stdout, stderr } = await cmd.output();
  const out = new TextDecoder().decode(stdout);
  const err = new TextDecoder().decode(stderr);
  if (code !== 0) {
    throw new Error(`git ${args.join(" ")} failed (${code}): ${err || out}`);
  }
  return out.trim();
}

async function patchJson(path: string, version: string): Promise<void> {
  const text = await Deno.readTextFile(path);
  const json = JSON.parse(text);
  json.version = version;
  await Deno.writeTextFile(path, JSON.stringify(json, null, 2) + "\n");
}

async function patchCargoVersion(path: string, version: string): Promise<void> {
  let text = await Deno.readTextFile(path);
  // Scope to the [package] table so dependency `version =` lines are never touched.
  const updated = text.replace(
    /^(\[package\][\s\S]*?\nversion\s*=\s*")([^"]*)(")/m,
    `$1${version}$3`,
  );
  if (updated === text) {
    throw new Error(
      `ci_version: could not find [package] version to patch in ${path}`,
    );
  }
  await Deno.writeTextFile(path, updated);
}

async function main(): Promise<void> {
  await git(["fetch", "--tags", "--force"]);
  const tagList = await git(["tag", "-l", "v*"]);
  const existingTags = tagList ? tagList.split("\n").filter(Boolean) : [];

  const { appVersion, releaseVersion, releaseTag } = computeVersion(new Date(), existingTags);

  if (dryRun) {
    console.log(`APP_VERSION=${appVersion}`);
    console.log(`RELEASE_VERSION=${releaseVersion}`);
    console.log(`RELEASE_TAG=${releaseTag}`);
    console.log(`(dry-run: no files patched, no GITHUB_ENV written)`);
    return;
  }

  await patchJson(FILES.tauriConf, appVersion);
  await patchJson(FILES.packageJson, appVersion);
  await patchCargoVersion(FILES.cargoToml, appVersion);

  const ghEnv = Deno.env.get("GITHUB_ENV");
  if (ghEnv) {
    const line = [
      `APP_VERSION=${appVersion}`,
      `RELEASE_VERSION=${releaseVersion}`,
      `RELEASE_TAG=${releaseTag}`,
      "",
    ].join("\n");
    await Deno.writeTextFile(ghEnv, line, { append: true });
  }

  console.log(`APP_VERSION=${appVersion}`);
  console.log(`RELEASE_VERSION=${releaseVersion}`);
  console.log(`RELEASE_TAG=${releaseTag}`);
}

main();
