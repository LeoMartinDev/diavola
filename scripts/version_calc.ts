export interface VersionResult {
  appVersion: string;
  releaseVersion: string;
  releaseTag: string;
}

export function formatVersion(version: string): VersionResult {
  return {
    appVersion: version,
    releaseVersion: version,
    releaseTag: `v${version}`,
  };
}
