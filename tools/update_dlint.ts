#!/usr/bin/env -S deno run --allow-write --allow-read --allow-net --allow-env
// Copyright 2018-2025 the Deno authors. All rights reserved. MIT license.
import {
  basename,
  extname,
  join,
} from "jsr:@std/path@^1.0.8";
import decompress from "npm:decompress@4.2.1";

const ORG = "denoland";
const REPO = "deno_lint";

const zipPaths = await downloadDenoLintZips(ORG, REPO);
await Promise.all(zipPaths.map(unzip));
console.log("Completed");

async function fetchLatestReleaseTag(
  org: string,
  repo: string,
): Promise<string> {
  const response = await fetch(
    `https://api.github.com/repos/${org}/${repo}/releases/latest`,
  );
  const release = await response.json();
  return release.tag_name;
}

type BinaryInfo = {
  filename: string;
  url: URL;
};

async function getBinariesInfo(
  org: string,
  repo: string,
): Promise<BinaryInfo[]> {
  const tag = await fetchLatestReleaseTag(org, repo);
  const downloadUrl =
    `https://github.com/denoland/deno_lint/releases/download/${tag}/`;
  return [
    {
      filename: "linux64.zip",
      url: new URL(
        "dlint-x86_64-unknown-linux-gnu.zip",
        downloadUrl,
      ),
    },
    {
      filename: "mac.zip",
      url: new URL("dlint-x86_64-apple-darwin.zip", downloadUrl),
    },
    {
      filename: "win.zip",
      url: new URL("dlint-x86_64-pc-windows-msvc.zip", downloadUrl),
    },
  ];
}

async function downloadAndSaveBinary(
  binary: BinaryInfo,
  targetDirPath: string,
) {
  console.log(`Downloading from ${binary.url}`);
  const response = await fetch(binary.url);
  if (!response.ok) {
    throw new Error(`error on fetch ${binary.url}: ${await response.text()}`);
  }
  const blob = await response.blob();
  const data = new Uint8Array(await blob.arrayBuffer());
  const filepath = join(targetDirPath, binary.filename);
  await Deno.writeFile(filepath, data);
  console.log(`Downloaded and saved ${binary.url} to ${filepath}`);
  return filepath;
}

async function downloadDenoLintZips(
  org: string,
  repo: string,
): Promise<string[]> {
  const tempDirPath = await Deno.makeTempDir();

  const binaries = await getBinariesInfo(org, repo);
  const paths = await Promise.all(
    binaries.map((b) => downloadAndSaveBinary(b, tempDirPath)),
  );
  console.log("All binaries have been downloaded.");
  return paths;
}

function unzipTargetDir(zipPath: string): string {
  const base = basename(zipPath);
  const dir = base.replace(extname(base), "");
  return join("prebuilt", dir);
}

async function unzip(zipPath: string) {
  console.log(`Unzipping ${zipPath}`);
  const targetDir = unzipTargetDir(zipPath);
  await decompress(zipPath, targetDir);
  console.log(`Unzipped ${zipPath} to ${targetDir}`);
}
