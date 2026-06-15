#!/bin/zsh
# Reverte o mod: remove nossos chunk*patch2.rpkg e restaura o packagedefinition original.
set -e
RES="$HOME/Library/Application Support/Steam/steamapps/common/HITMAN 3/Hitman WOA.app/Contents/Resources"
BK="$HOME/HitmanModBackup/packagedefinition.txt"

# Todos os chunkNpatch2.rpkg sao nossos (o Mac so vem com patch1 de fabrica).
n=$(ls "$RES"/chunk*patch2.rpkg 2>/dev/null | wc -l | tr -d ' ')
rm -f "$RES"/chunk*patch2.rpkg
echo "removidos $n patches (chunk*patch2.rpkg)"

if [ -f "$BK" ]; then
  cp "$BK" "$RES/packagedefinition.txt"
  echo "packagedefinition restaurado do backup"
else
  echo "AVISO: backup $BK nao encontrado — packagedefinition NAO restaurado"
fi
echo "Revertido."
