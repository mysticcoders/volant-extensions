/**
 * Bundles the TypeScript with esbuild, compiles it with Javy (QuickJS in WASM) into an ABI 2 WASI
 * command, gives its memory Volant's 16 MiB maximum, and pins the module's SHA-256 in manifest.json.
 * JAVY must name a Javy 9.1.0 executable; build.sh downloads and verifies one.
 */
import { build } from "esbuild";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { boundMemory } from "../tools/bound-memory.mjs";

const ENTRY = "src/main.ts";
const OUT = "out";
const BUNDLE = `${OUT}/npm.js`;
const COMPILED = `${OUT}/npm.wasm`;
const MANIFEST = "manifest.json";
const JAVY_OPTIONS = ["-C", "dynamic=n", "-C", "source=omitted", "-J", "text-encoding=y", "-J", "javy-stream-io=y", "-J", "event-loop=n"];

mkdirSync(OUT, { recursive: true });
await build({ entryPoints: [ENTRY], bundle: true, format: "esm", platform: "neutral", target: "es2020", outfile: BUNDLE, legalComments: "none" });
execFileSync(process.env.JAVY, ["build", BUNDLE, ...JAVY_OPTIONS, "-o", COMPILED], { stdio: "inherit" });
const manifest = JSON.parse(readFileSync(MANIFEST, "utf8"));
const module = boundMemory(readFileSync(COMPILED));
writeFileSync(manifest.module, module);
manifest.sha256 = createHash("sha256").update(module).digest("hex");
writeFileSync(MANIFEST, JSON.stringify(manifest, null, 2) + "\n");
