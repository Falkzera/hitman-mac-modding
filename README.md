# HITMAN World of Assassination — modding nativo no macOS (Apple Silicon)

Ferramentas e método para rodar mods do **HITMAN: World of Assassination** na **build NATIVA de macOS** (porte da Feral Interactive, via Steam) — **sem CrossOver/Wine para jogar**.

O caso de uso que motivou tudo: instalar a **tradução PT-BR da comunidade** no jogo nativo de Mac. Mas a técnica aqui (fazer mods feitos para Windows funcionarem no porte Mac do engine Glacier) serve para **qualquer** mod de conteúdo.

> **Funciona?** Sim. Menus, interface e **~17,4 mil legendas de diálogo** em português, rodando no app nativo (Apple Silicon). No teste do autor, o **online continuou funcionando** (progressão intacta).

---

## ⚠️ Aviso importante (leia antes)

- Este repositório contém **apenas ferramentas e documentação**. Ele **NÃO** distribui o jogo nem a tradução.
- Você precisa **ter o jogo** (Steam) e **baixar o mod você mesmo** no NexusMods (link abaixo).
- A técnica exige acesso aos **arquivos da versão Windows** do jogo (explico por quê e como mais abaixo).
- Mexer nos arquivos do jogo é **por sua conta e risco**. Faça backup. Sem afiliação com IO Interactive, Feral Interactive ou NexusMods.

---

## Compatibilidade (testado)

| Item | Versão testada |
|---|---|
| Jogo | HITMAN World of Assassination — app **3.270.1**, Steam **buildid 23678892** (appid 1659040) |
| Plataforma | macOS, **Apple Silicon (arm64)** — porte Feral (`Hitman WOA.app`, bundle `dk.ioi.ios.hitman`) |
| Mod | **HITMAN World of Assassination PT-BR v2.0.5** (release "3.270.1 Hotfix SMF") |
| Fonte do mod | https://www.nexusmods.com/hitman3/mods/1225 |
| Referência (Windows) | Mesma build **23678892** (precisa bater com o Mac e com o mod) |

> O método é **específico de versão**: se a build do jogo mudar, é preciso **regerar** os patches (mesmo procedimento).

---

## Por que isto não é trivial (a descoberta)

Mods de HITMAN são arquivos `.rpkg` que sobrescrevem recursos do jogo **pelo ID do recurso** (RuntimeResourceID — um hash do caminho do recurso). A intuição diz: o porte de Mac é o mesmo jogo, então os IDs deveriam ser iguais aos do Windows, e bastaria copiar o mod.

**Não é o caso.** Comparando os ~888 mil recursos do jogo de Mac com os ~890 mil do Windows (mesma build), descobrimos:

- **`mod ∩ Windows` = 18.228 / 18.228 (100%)** — o mod casa perfeitamente com o Windows.
- **`mod ∩ Mac` = 0** — em qualquer ordem de bytes.
- **`Windows ∩ Mac` = 0** — os dois jogos **não compartilham um único ID de recurso**, mesmo byte a byte.

Ou seja: o **porte da Feral usa um esquema de IDs de recurso diferente** do Windows. Um mod do Windows, copiado direto, aponta para IDs que **não existem** no jogo de Mac → é silenciosamente ignorado (o jogo abre normal, mas nada muda).

## A solução: *content-bridge* (o Windows como Pedra de Roseta)

Apesar de os **IDs** diferirem, o **conteúdo** dos recursos de localização (o texto inglês, descomprimido) é **byte-idêntico** entre Windows e Mac. Isso permite uma ponte:

```
recurso do mod (ID-Windows, texto PT)
   └─> acha o ORIGINAL inglês no Windows (mesmo ID)
         └─> casa esse conteúdo inglês com o do Mac  ──> descobre o ID-Mac
               └─> re-emite o texto PT sob o ID-Mac, no chunk certo
```

Assim nunca precisamos "quebrar" o hash da Feral — usamos o conteúdo como chave. Taxa de casamento:

| Tipo | Cobertura |
|---|---|
| **LOCR** (menus/UI/texto) | 639 / 648 (98,6%) |
| **DLGE** (legendas de diálogo) | 17.426 / 17.448 (99,87%) |
| **RTLV** (legendas de cutscene) | 128 / 132 — *o blob embute o ID do vídeo nos bytes 152–159; remapeamos esse ID* |
| **Total** | **~18.193 / 18.228 (99,8%)** |

Para DLGE (que referenciam áudio/animação), o patch usa o **blob de texto PT do mod** + a **tabela de referências do original do Mac** (os IDs de referência corretos do Mac). Validado: 18.065 recursos + 69.964 referências, **todos** existentes no jogo de Mac.

Detalhe técnico completo em [`docs/DEEP_DIVE.md`](docs/DEEP_DIVE.md).

---

## Pré-requisitos

1. **Mac Apple Silicon** com o **HITMAN WoA nativo** instalado (Steam).
2. **Rust** (toolchain): instale com [`rustup`](https://rustup.rs).
3. O **mod PT-BR** do NexusMods: https://www.nexusmods.com/hitman3/mods/1225 — baixe a **release standalone para Simple Mod Framework** (a que traz os arquivos `content/` precompilados) e extraia.
4. **Os arquivos da versão Windows do jogo** (a pasta `Runtime/` com os `.rpkg`), na **mesma build**. É a "Pedra de Roseta" para descobrir os IDs do Mac. Opções:
   - instalar o jogo via **CrossOver** só para obter os arquivos (você não precisa jogar por lá), ou
   - copiar a pasta `Runtime/` de um PC Windows, ou
   - baixar via Steam em outra máquina.

> Sem os arquivos Windows não dá para mapear os IDs do Mac. (Um mapa pré-computado de IDs eliminaria esse passo — veja [Roadmap](#roadmap).)

---

## Passo a passo

```sh
# 1) Toolchain Rust (se ainda não tiver)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# 2) Clonar e compilar
git clone https://github.com/Falkzera/hitman-mac-modding.git
cd hitman-mac-modding
cargo build --release

# 3) Localize os três caminhos (ajuste se necessário):
MAC="$HOME/Library/Application Support/Steam/steamapps/common/HITMAN 3/Hitman WOA.app/Contents/Resources"
WIN="$HOME/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/HITMAN 3/Runtime"
MOD="$HOME/Downloads/HITMAN-World-of-Assassination-PTBR"   # pasta extraída do mod (contém content/)

# 4) Gerar os patches remapeados (LOCR + DLGE) em build/remap-full/
./target/release/remap "$MAC" "$WIN" "$MOD" "$PWD/build/remap-full"

# 5) (Opcional) validar: todo ID de recurso e de referência deve existir no Mac
./target/release/vfull "$MAC" "$PWD/build/remap-full"

# 6) BACKUP do arquivo de config antes de instalar
mkdir -p ~/HitmanModBackup
cp "$MAC/packagedefinition.txt" ~/HitmanModBackup/

# 7) Instalar (copia os patches + sobe o patchlevel das partições)
./install.sh

# 8) Reverter quando quiser
./uninstall.sh
```

Antes de jogar:

- Steam → HITMAN WoA → **Propriedades → Atualizações → "Só atualizar quando eu iniciar"** (evita updates reverterem o mod).
- **Não** rode "Verificar integridade dos arquivos" com o mod instalado (reverte tudo).
- **Idioma do jogo em English** — o mod sobrescreve o slot de inglês com o texto PT.

---

## Online

No teste do autor, **o jogo conectou online normalmente** com o mod instalado — porque o patch só troca o **texto** dos recursos (mantendo os IDs do Mac), e isso não foi sinalizado como conteúdo adulterado. Resultado: progressão e features online preservadas.

> Sem garantias: comportamento pode variar por conta/região e a IOI/Feral pode mudar a verificação a qualquer momento. Se preferir 100% de segurança, jogue **offline**. Use por sua conta e risco.

---

## Limitações

- **Legendas de cutscene (RTLV):** 128/132 traduzidas. O blob do RTLV embute *inline* o ID do vídeo (bytes 152–159), que difere entre plataformas — nós remapeamos esse ID. Sobram ~35 recursos (alguns LOCR/DLGE/RTLV) que não casam por conteúdo e ficam em inglês (~0,2% do mod).
- **Específico da build 23678892.** Outra versão do jogo/mod exige regerar os patches (mesmo procedimento).
- **Updates da Steam revertem o mod** — rode `./install.sh` de novo depois.
- **Precisa dos arquivos Windows** para gerar (veja Roadmap).
- Não toca no executável do app (só em `Contents/Resources/`), então a assinatura ad-hoc do app **não** é invalidada.

---

## As ferramentas (`src/bin/`)

`remap` é a ferramenta principal; o resto foi escrito durante a investigação e fica como diagnóstico/reprodutibilidade.

| Bin | Função |
|---|---|
| **`remap`** | **Gera os patches do Mac** remapeando o mod (LOCR + DLGE) via content-bridge. |
| `verify` | Reabre um patch e confere byte a byte contra a fonte. |
| `vfull` | Valida que todo ID de recurso e referência dos patches existe no Mac. |
| `bridge` | Mede a taxa de casamento Windows↔Mac por conteúdo (viabilidade). |
| `distribute` / `ownership` / `wherein` | Em que chunk/arquivo cada recurso vive. |
| `fullcheck` / `probe` / `hashcheck` | Comparação de conjuntos de IDs e validação do hash IOI. |
| `types` / `chunkmap` / `diag` | Tipos de recurso e diagnóstico de montagem de partições. |

`install.sh` / `uninstall.sh`: instalam/revertem (copiam patches + ajustam `packagedefinition.txt`).

---

## Roadmap

- [ ] Publicar um **mapa pré-computado `ID-Windows → ID-Mac`** (só pares de hash, sem conteúdo do jogo) para que outros usuários **não precisem dos arquivos Windows**.
- [ ] Investigar a ponte para **RTLV** (legendas de cutscene).
- [ ] Generalizar para outras builds / outros mods de conteúdo.

---

## Créditos

- **Tradução PT-BR:** FrankieScheuer e ImPedrooooo — [NexusMods mod 1225](https://www.nexusmods.com/hitman3/mods/1225). Todo o trabalho de tradução é deles.
- **[`rpkg-rs`](https://github.com/dafitius/rpkg-rs)** (dafitius / glacier-modding) — leitura/escrita do formato RPKG.
- Comunidade **[glacier-modding](https://github.com/glacier-modding)** e o **Simple Mod Framework**.
- IO Interactive e Feral Interactive pelo jogo e pelo porte (sem afiliação).

## Licença

Código sob [MIT](LICENSE). A licença cobre **apenas** as ferramentas deste repositório — não o jogo nem a tradução.
