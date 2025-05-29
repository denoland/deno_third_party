import { dirname, join, normalize } from "jsr:@std/path";
import { UntarStream } from "jsr:@std/tar/untar-stream";

async function getLatestVersion() {
  const response = await fetch(
    "https://registry.npmjs.org/@esbuild/linux-x64",
  );
  const json = await response.json();
  const latest = json["dist-tags"].latest;
  if (typeof latest !== "string") {
    throw new Error("Failed to get latest version");
  }
  return latest;
}

function infoForPlatform(platform: string) {
  let dir;
  let binName;
  let suffix;

  if (platform.endsWith("x64")) {
    binName = "esbuild-x64";
  } else if (platform.endsWith("arm64")) {
    binName = "esbuild-aarch64";
  } else {
    throw new Error(`Unsupported platform: ${platform}`);
  }

  if (platform.startsWith("win")) {
    dir = "win";
    suffix = ".exe";
  }
  if (platform.startsWith("linux")) {
    dir = "linux64";
    suffix = "";
  }
  if (platform.startsWith("darwin")) {
    dir = "mac";
    suffix = "";
  }
  if (dir === undefined || binName === undefined || suffix === undefined) {
    throw new Error(`Unsupported platform: ${platform}`);
  }
  return { dir, binName, suffix };
}

const platforms = [
  "darwin-arm64",
  "darwin-x64",
  "linux-x64",
  "linux-arm64",
  "win32-x64",
];

const latest = await getLatestVersion();

console.log("downloading esbuild on version", latest);

for (const platform of platforms) {
  const { dir, binName, suffix } = infoForPlatform(platform);
  const url =
    `https://registry.npmjs.org/@esbuild/${platform}/-/${platform}-${latest}.tgz`;
  const response = await fetch(url);
  const outPath = join(
    import.meta.dirname!,
    "../prebuilt",
    dir,
    `${binName}${suffix}`,
  );
  const outDir = dirname(outPath);
  await Deno.mkdir(outDir, { recursive: true });
  await using file = await Deno.open(
    outPath,
    {
      write: true,
      create: true,
    },
  );
  const entries = response.body?.pipeThrough(
    new DecompressionStream("gzip"),
  ).pipeThrough(new UntarStream()).values();
  if (!entries) {
    throw new Error("Failed to decompress tarball");
  }
  for await (const entry of entries) {
    const path = normalize(entry.path);
    console.log(path);

    if (path === "package/bin/esbuild" || path === "package/esbuild.exe") {
      await entry.readable?.pipeTo(file.writable);
      break;
    }
  }
  await Deno.chmod(outPath, 0o755);
}
