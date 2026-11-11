#!/usr/bin/env python3
"""Download the pinned forge-rs and crates.io sources into vendor/. The only build step that uses the network.

forge-rs files are checked against dependencies.lock.json; crates against Cargo.lock checksums by cargo.
"""
import hashlib, io, json, os, subprocess, tarfile, urllib.request
from pathlib import Path
from paths import MOD, CONFIG

FORGE_RS='https://codeload.github.com/Fexty12573/forge-rs/tar.gz/'

def forge_rs(rev,pins,dest):
    with urllib.request.urlopen(FORGE_RS+rev,timeout=120) as response: data=response.read()
    found={}
    with tarfile.open(fileobj=io.BytesIO(data),mode='r:gz') as archive:
        for entry in archive.getmembers():
            name='/'.join(Path(entry.name).parts[1:])  # drop the forge-rs-<rev>/ prefix
            if entry.isfile() and name in pins: found[name]=archive.extractfile(entry).read()
    for name,expected in pins.items():
        if name not in found or hashlib.sha256(found[name]).hexdigest()!=expected:
            raise ValueError('forge-rs '+rev+' differs from the pin: '+name)
    for name,data in found.items():
        path=dest/name;path.parent.mkdir(parents=True,exist_ok=True);path.write_bytes(data)

def main():
    lock=json.loads((MOD/'dependencies.lock.json').read_text())
    forge,crates=MOD/'vendor/forge-rs',MOD/'vendor/crates'
    for p in (forge,crates):
        if p.exists(): raise ValueError('Preserving existing '+str(p.relative_to(MOD))+'; remove it to fetch again')
    prefix='vendor/forge-rs/'
    forge_rs(lock['forge_rs_revision'],{n[len(prefix):]:h for n,h in lock['files'].items() if n.startswith(prefix)},forge)
    rust=Path(CONFIG['rust_bin'])
    library=(rust.parent/'lib/rustlib/src/rust/library').resolve()  # cargo rejects a workspace path with '..'
    # build-std resolves the toolchain's library workspace, and test harnesses keep their own locks.
    extra=[library/'Cargo.toml',*(p.parent/'Cargo.toml' for p in sorted(MOD.glob('tests/*/Cargo.lock')))]
    env=dict(os.environ,CARGO_HOME=str(CONFIG['cargo_home']),CARGO_NET_OFFLINE='false')
    args=[rust/'cargo','vendor','--quiet','--locked','--versioned-dirs']
    for manifest in extra: args+=['--sync',manifest]
    subprocess.run([*map(str,args),'vendor/crates'],cwd=MOD,env=env,check=True,stdout=subprocess.DEVNULL)
    print('Fetched forge-rs',lock['forge_rs_revision'],'and crates into vendor/')

if __name__=='__main__': main()
