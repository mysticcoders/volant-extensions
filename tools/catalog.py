#!/usr/bin/env python3
"""Builds the Volant extension catalog site from every folder holding manifest.json and listing.json.

The site has catalog.json, the index Volant reads, and packages/<id>/<version>/ with each manifest,
module and optional WebP icon. CI signs the exact bytes of catalog.json with Ed25519 into
catalog.json.sig (base64) and publishes the site to GitHub Pages. Volant derives package paths from the
ID, version and file names rather than trusting paths in the index, and checks every byte against the
sizes and hashes listed here. Volant 0.3.0 ignores the icon fields and downloads only the other two files.

revoked.json, when present, lists withdrawn extensions as [{"id", "sha256"?, "reason"?}]: without sha256
every version of the ID is revoked, with it only that module. Volant turns off installed copies that
match and refuses to enable or install them. A revoked listing is left out of the catalog.

Usage: tools/catalog.py OUT_DIR
"""
import hashlib
import json
import pathlib
import re
import shutil
import subprocess
import sys

REPOSITORY = "https://github.com/mysticcoders/volant-extensions"
ID = re.compile(r"^[a-z0-9-]+(\.[a-z0-9-]+)+$")
VERSION = re.compile(r"^[0-9A-Za-z.+-]{1,32}$")
MODULE = re.compile(r"^[A-Za-z0-9_-]{1,64}\.wasm$")
ICON = re.compile(r"^[A-Za-z0-9_-]{1,64}\.webp$")
MAX_ICON_BYTES = 32768
SHA256 = re.compile(r"^[0-9a-f]{64}$")
MAX_REVOCATIONS = 256


def git(*args):
    return subprocess.check_output(["git", *args], text=True).strip()


def entry(data):
    return {"sha256": hashlib.sha256(data).hexdigest(), "size": len(data)}


def load_revocations(root):
    path = root / "revoked.json"
    if not path.exists():
        return []
    revoked = json.loads(path.read_text())
    if not isinstance(revoked, list) or len(revoked) > MAX_REVOCATIONS:
        sys.exit(f"revoked.json must be a list of at most {MAX_REVOCATIONS} entries")
    for item in revoked:
        if not isinstance(item, dict) or set(item) - {"id", "sha256", "reason"}:
            sys.exit(f"revoked.json: unexpected entry {item!r}")
        ident, digest, reason = item.get("id"), item.get("sha256"), item.get("reason")
        if not (isinstance(ident, str) and ID.match(ident) and len(ident) <= 128):
            sys.exit(f"revoked.json: {ident!r} is not a catalog-safe id")
        if digest is not None and not (isinstance(digest, str) and SHA256.match(digest)):
            sys.exit(f"revoked.json: {ident} sha256 must be 64 lowercase hex characters")
        if reason is not None and not (isinstance(reason, str) and len(reason) <= 300):
            sys.exit(f"revoked.json: {ident} reason must be text of at most 300 characters")
    return revoked


def is_revoked(revoked, ident, digest):
    return any(item["id"] == ident and item.get("sha256") in (None, digest) for item in revoked)


def main(out):
    root = pathlib.Path(__file__).resolve().parent.parent
    revision = git("rev-parse", "HEAD")
    site = pathlib.Path(out)
    if site.exists():
        shutil.rmtree(site)
    revoked = load_revocations(root)
    listings = []
    for listing_path in sorted(root.glob("*/listing.json")):
        folder = listing_path.parent
        manifest_bytes = (folder / "manifest.json").read_bytes()
        manifest = json.loads(manifest_bytes)
        listing = json.loads(listing_path.read_text())
        ident, version, module = manifest["id"], manifest["version"], manifest["module"]
        if not (ID.match(ident) and len(ident) <= 128 and VERSION.match(version) and MODULE.match(module)):
            sys.exit(f"{folder.name}: id, version or module name is not catalog-safe")
        module_bytes = (folder / module).read_bytes()
        if hashlib.sha256(module_bytes).hexdigest() != manifest["sha256"]:
            sys.exit(f"{folder.name}: manifest sha256 does not match {module}")
        if is_revoked(revoked, ident, manifest["sha256"]):
            print(f"{folder.name}: revoked, not listed")
            continue
        package = site / "packages" / ident / version
        package.mkdir(parents=True)
        (package / "manifest.json").write_bytes(manifest_bytes)
        (package / module).write_bytes(module_bytes)
        icon = manifest.get("icon")
        icon_fields = {}
        if icon:
            icon_bytes = (folder / icon).read_bytes()
            if not ICON.match(icon) or len(icon_bytes) > MAX_ICON_BYTES or icon_bytes[:4] != b"RIFF" or icon_bytes[8:12] != b"WEBP":
                sys.exit(f"{folder.name}: the icon must be a WebP of at most 32 KB with a catalog-safe name")
            if hashlib.sha256(icon_bytes).hexdigest() != manifest.get("iconSha256"):
                sys.exit(f"{folder.name}: manifest iconSha256 does not match {icon}")
            (package / icon).write_bytes(icon_bytes)
            icon_fields = {"icon": icon, "iconFile": entry(icon_bytes)}
        listings.append({
            "id": ident, "name": manifest["name"], "version": version, "module": module,
            "summary": listing["summary"], "publisher": listing["publisher"], "license": listing["license"],
            "notes": listing.get("notes"), "abiVersion": manifest.get("abiVersion", 1),
            "capabilities": manifest.get("capabilities", []), "hosts": manifest.get("hosts", []),
            "source": f"{REPOSITORY}/tree/{revision}/{folder.name}",
            "manifest": entry(manifest_bytes), "moduleFile": entry(module_bytes), **icon_fields,
            **({"storage": True} if manifest.get("storage") is True else {}),
        })
    if len({item["id"] for item in listings}) != len(listings):
        sys.exit("duplicate extension IDs")
    catalog = {
        "format": 1,
        "sequence": int(git("rev-list", "--count", "HEAD")),
        "generated": git("show", "-s", "--format=%cI", "HEAD"),
        "revision": revision,
        "extensions": listings,
        **({"revoked": revoked} if revoked else {}),
    }
    site.mkdir(parents=True, exist_ok=True)
    (site / "catalog.json").write_text(json.dumps(catalog, indent=2, sort_keys=True) + "\n")
    (site / ".nojekyll").write_text("")
    print(f"catalog {catalog['sequence']} at {revision[:12]}: " + ", ".join(f"{i['id']} {i['version']}" for i in listings)
          + (f"; {len(revoked)} revoked" if revoked else ""))


if __name__ == "__main__":
    main(sys.argv[1] if len(sys.argv) > 1 else "site")
