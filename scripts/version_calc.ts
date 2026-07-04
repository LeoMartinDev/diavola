export interface VersionResult {
  appVersion: string;
  releaseVersion: string;
  releaseTag: string;
}

export function computeVersion(now: Date, existingTags: string[]): VersionResult {
  const year = now.getFullYear();
  const yy = year - 2000;
  const month = now.getMonth() + 1;
  const day = now.getDate();

  const releasePrefix = `${year}.${String(month).padStart(2, "0")}.${String(day).padStart(2, "0")}`;
  const tagPrefix = `v${releasePrefix}.`;

  let maxPatch = 0;
  for (const tag of existingTags) {
    if (!tag.startsWith(tagPrefix)) continue;
    const suffix = tag.slice(tagPrefix.length);
    const patch = Number.parseInt(suffix, 10);
    if (String(patch) === suffix && patch > maxPatch) {
      maxPatch = patch;
    }
  }

  const patch = maxPatch + 1;
  return {
    appVersion: `${yy}.${month}.${day}-${patch}`,
    releaseVersion: `${releasePrefix}.${patch}`,
    releaseTag: `v${releasePrefix}.${patch}`,
  };
}
