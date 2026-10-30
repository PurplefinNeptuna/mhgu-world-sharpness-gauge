#!/usr/bin/env python3
"""Check the staged candidate: native imports, exact code signatures, the hook
window, the flag-rule audit and the payload inventory. Static analysis only;
it proves nothing about pixels, which need a user run."""
import ast,json,re,struct,subprocess
from pathlib import Path
from paths import MOD,CONFIG,BUILD_ROOT,CARGO_TARGET,VERSION,input_path
from build import DRAWING_CHANGED,doctor,digest,record,source_hashes
from mhgu_binary import Nro,main_image
from native_support import nso_exports

NRO='modern_sharpness_gauge.nro'
HOOK=0xa494a0
# push {r4,r5,r11,lr} then mov r4,r1: a function entry with no PC-relative
# operand, so Forge can relocate the first eight bytes into its pool.
HOOK_BYTES=bytes.fromhex('30482de9'+'0140a0e1')
SIGNATURE_WINDOW=[bytes.fromhex(w) for w in
    ['30482de9','0140a0e1','801090e5','000051e3','3088bd08','8c20d0e5','010012e3','4000000a',
     '682090e5','000052e3','3700000a','00c0a0e3']]
def symbols():
    elf=CARGO_TARGET/'armv7a-none-eabi/release/libmodern_sharpness_gauge.so'
    result={}
    for line in subprocess.check_output([Path(CONFIG['devkitarm'])/'bin/arm-none-eabi-nm','-n',elf],text=True).splitlines():
        fields=line.split()
        if len(fields)==3: result[fields[2]]=int(fields[0],16)
    return elf,result
def verify():
    doctor();nro_path=BUILD_ROOT/('package/romfs/nativeNX/plugins/'+NRO)
    data=nro_path.read_bytes();nro=Nro(data);elf,names=symbols()
    # DT_TEXTREL / DF_TEXTREL require writes to memory the Switch maps read-only.
    if 22 in nro.tags or nro.tags.get(30,0)&4:
        raise ValueError('NRO requires dynamic relocations in read-only memory')
    receipt=json.loads((BUILD_ROOT/'build.json').read_text())
    if receipt.get('version')!=VERSION or receipt['nro_sha256']!=digest(nro_path) \
       or receipt.get('source_sha256')!=source_hashes() or receipt.get('drawing_changed') is not DRAWING_CHANGED:
        raise ValueError('Build receipt differs from the staged NRO, current sources, or claims a drawing change')
    exports={n.decode() for n,e,*_ in nro.symbols if n and e[5] and e[3]>>4==1}
    # No settings, so no Plugin UI: the build exports no ImGui callbacks at all.
    if exports!={'forge_onLoad','forge_onInit','forge_onUpdate','init','fini'}:
        raise ValueError('Unexpected plugin exports: '+str(exports))
    imports={n.decode() for n,e,*_ in nro.symbols if n and not e[5] and e[3]>>4==1}
    available=set().union(*(nso_exports(input_path(n)) for n in ['main','sdk','forge']))
    if imports-available: raise ValueError('Unresolved native imports: '+str(imports-available))
    image,_=main_image(input_path('main').read_bytes());windows=[]
    for offset,raw in re.findall(r'\(\s*0x([0-9a-f]+),\s*\[([^\]]+)\]',(MOD/'src/signatures.rs').read_text()):
        offset=int(offset,16);expected=ast.literal_eval('['+raw+']')
        if image[offset:offset+32]!=bytes(expected): raise ValueError('Signature mismatch: '+hex(offset))
        windows.append(hex(offset))
    if len(windows)<12: raise ValueError('Supporting signature windows missing')
    if image[HOOK:HOOK+8]!=HOOK_BYTES: raise ValueError('Hook target prologue differs')
    # The full 12-word window must be unique, or the hook can land on a twin.
    pattern=b''.join(SIGNATURE_WINDOW)
    if image.count(pattern)!=1: raise ValueError('Hook signature is not unique: %d matches' % image.count(pattern))
    # The Phase 0 instruction audit found no SEL/USAD8/MRS/MSR producer and no
    # APSR_ge writer in the window; the flag-consuming popeq sits inside the
    # relocated unit, which is why the window covers main+0xa494a0..0xa494cc.
    # The local-player chain cells (main+0x188ca48 session, main+0x1885b58
    # sPlayer) live past the end of the NSO file image, in .bss, so no file
    # bytes exist to compare. They are proven from the Ghidra decompilations
    # recorded in work/modern-sharpness-gauge/PHASE0_RESEARCH.md and each is
    # read through the checked mapping in src/memory.rs at runtime.
    if len(image)>0x188ca48: raise ValueError('Chain cells moved into the file image; add byte checks')
    disasm=subprocess.check_output([Path(CONFIG['devkitarm'])/'bin/arm-none-eabi-objdump','-d',elf],text=True)
    writes=[line for line in disasm.splitlines() if '\tmsr\t' in line]
    if writes: raise ValueError('Processor status writes are forbidden (NZCVQ/GE rule): '+str(writes))
    # The Phase 4 closing removed the Plugin UI panel (no settings, so no panel), so no ImGui
    # widget import is expected at all. Persistent settings stay out: no config creation, no
    # save-data mount, no colour editor.
    if imports&{'forge_config_create','igColorEdit3','forge_fs_mountSaveData'}:
        raise ValueError('Unrequested settings/overlay dependency')
    if 'forge_onImGuiFreeRender' in exports: raise ValueError('No free rendering is used by this mod')
    expected={'romfs/nativeNX/plugins/'+NRO}
    source=BUILD_ROOT/'package';actual={str(p.relative_to(source)) for p in source.rglob('*') if p.is_file()}
    if actual!=expected: raise ValueError('Unexpected runtime payload files: '+str(actual))
    record('verify.json',{'status':'PASSED','nro_sha256':digest(nro_path),'elf_sha256':digest(elf),
        'exports':sorted(exports),'strong_imports':sorted(imports),'signature_windows':windows,
        'hook_address':hex(HOOK),'hook_window_words':len(SIGNATURE_WINDOW),'hook_signature_unique_matches':1,
        'hook_abi_bytes':24,'forge_trampoline_bytes':32,'forge_pool_chunk_bytes':16384,
        'compiled_status_writes':writes,'local_player_cells':['main+0x188ca48 session','main+0x1885b58 sPlayer (.bss, Ghidra-proven)'],
        'runtime_active':True,'drawing_changed':DRAWING_CHANGED,'text_relocations':False,
        'observes':['gauge controller identity','local player pointer','local player sharpness fields'],
        'runtime_test':'NOT RUN'})
    print('PASS: native imports/signatures, hook window uniqueness, flag-rule audit and payload inventory')
if __name__=='__main__': verify()
