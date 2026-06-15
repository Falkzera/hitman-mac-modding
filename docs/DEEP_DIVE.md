# Deep dive — como descobrimos e resolvemos

A história técnica de como saímos de "o mod não funciona no Mac" para "99% traduzido, nativo, com online funcionando". Documentado porque o caminho (e os becos sem saída) é tão útil quanto a solução.

## 1. O layout do porte Mac

- Os `.rpkg` ficam em `Hitman WOA.app/Contents/Resources/` (no Windows é `Runtime/`).
- Formato **RPKGv2** (magic `2KPR`) — o mesmo do Windows.
- `packagedefinition.txt` é **texto plano** no Mac (no Windows é cifrado com XTEA). Define partições (`super`=chunk0, `base`=chunk1, … posicionalmente até chunk30) e o `patchlevel` de cada uma.
- O app é **ad-hoc signed**, com **`Sealed Resources=none`** — mexer em `Resources/` não invalida a assinatura (desde que não se toque no executável em `Contents/MacOS/`).
- chunk0/chunk1 têm base **multi-parte** (`chunk0part1..6`, `chunk1part1..3`); os demais são arquivo único. (Detalhe relevante: o `PartitionManager` do `rpkg-rs` não monta multi-parte — use `ResourcePackage::from_file` por arquivo.)

## 2. Primeira tentativa (ingênua) — e por que falhou

O mod (release SMF) traz 18.228 recursos **precompilados** (17.448 DLGE + 648 LOCR + 132 RTLV), cada um com `.meta.json` (hash_value, tipo, referências).

Construímos um `chunk0patch2.rpkg` com `rpkg-rs` (`PackageBuilder` + `PatchId::Patch(2)`) usando os `hash_value` do mod como IDs, copiamos para `Resources/` e subimos `super` para `patchlevel=2`.

**Resultado:** jogo abre, conecta online, **nenhuma tradução**. O patch foi ignorado.

## 3. A conclusão errada (e a crítica correta)

Uma análise comparou os 18.228 IDs do mod com os ~888 mil do Mac: **0 correspondências**. Conclusão precipitada: "a Feral re-hasheou tudo, é inviável".

A crítica certa (do Falcão) foi: *isso pode ser bug de **byte-order/representação**, não diferença real de esquema* — e a navalha de Occam favorece **uma** causa (ordem de bytes no escritor E na comparação) em vez de "a Feral renomeou todos os caminhos". A auto-verificação anterior era cega ao bug (a mesma lib relendo o que ela mesma escreveu).

## 4. A Pedra de Roseta: a versão Windows

Instalamos o jogo Windows (via CrossOver, **só como fonte de dados, read-only**) e fizemos a comparação de três vias, comparando **bytes crus**, as-is **e** byte-revertido:

| Interseção | as-is | byte-revertido |
|---|---:|---:|
| mod ∩ Windows | 0 | **18.228 / 18.228** |
| mod ∩ Mac | 0 | **0** |
| Windows ∩ Mac | 0 | **0** |
| IOI-hash(paths) ∩ Windows | 0 | **255 / 255** |
| IOI-hash(paths) ∩ Mac | 0 | 0 |

Isso **descartou o byte-order** (o mod casa 100% com o Windows quando byte-revertido; com o Mac, 0 em qualquer ordem) e **provou a diferença real de esquema** (`Windows ∩ Mac = 0` byte a byte). O hash IOI (`md5` truncado) foi validado: reproduz os IDs do Windows (255/255), mas não os do Mac. Confirmação independente: um parser de RPKG escrito do zero em Python concordou exatamente com o `rpkg-rs` (888.076 IDs; 0/18.228).

> Lição: nunca confiar numa única lib para a verdade-base. Dois parsers independentes + uma referência externa (Windows) é o que fechou a questão.

## 5. A virada: bridge por conteúdo

Os **IDs** diferem, mas o **conteúdo inglês descomprimido** é byte-idêntico entre Windows e Mac. Medimos (ferramenta `bridge`): LOCR 639/648, DLGE 17.426/17.448, RTLV 0/132.

Pipeline (`remap`):
1. Para cada recurso do mod (ID-Windows), ler o **original inglês no Windows** (mesmo ID).
2. Casar esse conteúdo com o do Mac → descobrir o **ID-Mac** e o **chunk**.
3. Emitir o **blob PT do mod** sob o **ID-Mac**. Para DLGE, anexar as **referências do original do Mac** (as refs ficam no meta, separadas do blob; como o blob inglês é idêntico entre plataformas, a indexação das refs casa).
4. Agrupar por chunk → um `chunkNpatch2.rpkg` por chunk → subir `patchlevel=2` nas partições afetadas.

Validação (`vfull`): 18.065 recursos + 69.964 referências, **todos** existentes no Mac (0 fora).

## 6. Confirmação in-game

Primeiro teste: trocar só o `chunk0patch2.rpkg` (597 LOCR de menu) pelo remapeado. **Menu apareceu em português** → provou que (a) a engine carrega `patch2` via `patchlevel` e (b) o bridge funciona. Depois, o conjunto completo (29 chunks, LOCR + DLGE) → diálogo em PT dentro das missões, online funcionando.

## 7. Por que NÃO mexemos no `partitionmap.txt`

Suspeitávamos que o `partitionmap.txt` (manifesto com md5 por chunk) precisasse listar o novo patch. Não precisou: a engine descobre os patches pelo padrão de nome `chunkNpatch{M}.rpkg` governado pelo `patchlevel` do `packagedefinition`. O menu traduzido sem tocar no partitionmap confirmou isso.
