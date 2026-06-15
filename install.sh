#!/bin/zsh
# (Re)instala o mod PT-BR remapeado no Hitman WoA nativo (macOS).
# Util depois de um update da Steam que reverta o mod.
set -e
RES="$HOME/Library/Application Support/Steam/steamapps/common/HITMAN 3/Hitman WOA.app/Contents/Resources"
FULL="$HOME/Projects/HitmanMod/build/remap-full"
PD="$RES/packagedefinition.txt"

[ -d "$RES" ] || { echo "Resources nao encontrado: $RES"; exit 1; }
[ -d "$FULL" ] || { echo "patches nao encontrados: $FULL"; exit 1; }

# Guardrail: nunca instalar vazio (evita apagar a traducao por engano).
N=$(ls "$FULL"/*.rpkg 2>/dev/null | wc -l | tr -d ' ')
[ "$N" -gt 0 ] || { echo "ERRO: nenhum .rpkg em $FULL — abortando (nada a instalar)."; exit 1; }
echo "Copiando $N patches..."
cp "$FULL"/*.rpkg "$RES/"

echo "Subindo patchlevel=2 nas particoes dos chunks com patch..."
python3 - "$PD" "$FULL" <<'PY'
import sys, os, glob, re
pd, full = sys.argv[1], sys.argv[2]
targets={int(re.match(r"chunk(\d+)patch2", os.path.basename(p)).group(1))
         for p in glob.glob(os.path.join(full,"chunk*patch2.rpkg"))}
data=open(pd,"rb").read(); lines=data.split(b"\r\n"); idx=-1; ch=0
for i,ln in enumerate(lines):
    if ln.startswith(b"@partition"):
        idx+=1
        if idx in targets and b"patchlevel=" in ln:
            lines[i]=re.sub(rb"patchlevel=\d+", b"patchlevel=2", ln); ch+=1
open(pd,"wb").write(b"\r\n".join(lines))
print(f"  particoes em patchlevel=2: {ch}")
PY
echo "OK. Lembre: Steam offline + idioma English."
