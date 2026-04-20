# RPI Open Emulator (Rust)

Launcher desktop open-source para biblioteca local de ROMs com **RetroArch/libretro** (MVP em evolucao).

Este diretorio implementa a decisao do plano para:

- stack de UI Rust: **egui + eframe**;
- formato de configuracao inicial: **TOML** com serializacao via `serde`;
- scanner de ROM, resolucao de core e execucao RetroArch;
- catalogo SQLite, recentes, favoritos e metadata offline-first;
- integracao desktop (`.desktop` + icone).

## Nome do crate e pasta do repositorio

- **Nome do pacote Rust / binario:** `rpi_open_emulator` (definido em `Cargo.toml`).
- **Pasta do projeto:** pode ser `rpi_open_emulator` no disco (recomendado alinhar com o nome do crate).

## Migracao a partir da versao beta (`rpi5-launcher`)

No Linux, com `directories` 5, os dados ficam em:

- `~/.config/rpi_open_emulator/config.toml`
- `~/.local/share/rpi_open_emulator/catalog.sqlite3`

Na **primeira execucao** desta versao, se ainda nao existir config nova mas existir a antiga, o app **copia automaticamente**:

- de `~/.config/rpi5-launcher/config.toml`
- e `~/.local/share/rpi5-launcher/catalog.sqlite3`

para os caminhos novos acima (os arquivos antigos permanecem; voce pode apagar manualmente depois).

Se voce tinha instalado o `.desktop` antigo (`rpi5-launcher.desktop`), remova-o ou instale o novo com `./packaging/install-desktop.sh` para evitar duas entradas no menu.

## Stack escolhida

- `egui` + `eframe`: ciclo rapido para MVP desktop, sem depender de webview.
- `serde` + `toml`: tipagem forte no codigo e arquivo de config legivel.
- `directories`: caminho padrao de configuracao por usuario no Linux.

## Formato de configuracao

Arquivo principal esperado em:

- Linux: `~/.config/rpi_open_emulator/config.toml` (via `directories::ProjectDirs`)

Estrutura base:

- `retroarch`: binario, pasta de cores e argumentos globais.
- `library`: pastas de ROMs e BIOS.
- `systems.<plataforma>`: `default_core`, extensoes aceitas e args extras.

Exemplo completo em `config/default.toml`.

## Execucao local

```bash
cd rpi_open_emulator
cargo run
```

Ao iniciar, o app cria config padrao se ela nao existir.

## Build: o que e instalado automaticamente e o que nao e

### O Cargo **nao** instala pacotes do sistema (apt/snap)

Ao rodar `cargo build` ou `cargo run` pela primeira vez:

- O **Cargo** baixa e compila **crates Rust** a partir do [crates.io](https://crates.io) (dependencias declaradas em `Cargo.toml` e toda a arvore transitiva registrada em `Cargo.lock`).
- O Cargo **nao** executa `apt install`, **nao** instala **RetroArch** e **nao** instala pacotes **libretro** (cores `.so` do sistema). Isso e responsabilidade sua (ou do instalador da distro).
- O projeto **nao** empacota nem baixa ROMs, BIOS nem cores libretro: apenas orquestra o RetroArch que ja estiver instalado.

### Crates Rust usados diretamente pelo binario

Declarados em `Cargo.toml` (versoes fixadas pelo `Cargo.lock` ao compilar):

| Crate | Funcao resumida |
|-------|-----------------|
| `anyhow` | Erros e contexto |
| `directories` | Caminhos XDG (`~/.config`, `~/.local/share`) |
| `eframe` / `egui` | Janela desktop e UI |
| `rusqlite` (feature `bundled`) | SQLite embutido: compila o SQLite em C **durante** o `cargo build` |
| `serde` | Serializacao da config TOML |
| `toml` | Parser/gerador TOML |

Arvore completa (centenas de crates transitivos, incluindo `winit`, `wgpu`/OpenGL, etc.):

```bash
cd rpi_open_emulator
cargo tree
```

### Dependencias nativas exigidas **no seu sistema** para compilar

Sem isso, o `cargo build` pode falhar ao compilar `libsqlite3-sys` (via `rusqlite` + `bundled`) ou ao linkar a UI:

- **Compilador C** (`build-essential` no Debian/Ubuntu/Raspberry Pi OS) — necessario para compilar o SQLite embutido.
- **Ferramentas de build**: `pkg-config` (e as vezes `cmake`) conforme o ambiente.
- **Bibliotecas de desenvolvimento da UI** (Linux desktop): nomes variam por distro; se o link falhar, instale os pacotes `-dev` indicados pelo erro (comuns em desktop X11/Wayland: X11, mesa/GL, ALSound).

Exemplo **ilustrativo** para Debian/Ubuntu/Raspberry Pi OS (ajuste se o `apt` sugerir nomes diferentes):

```bash
sudo apt update
sudo apt install -y \
  build-essential pkg-config \
  libx11-dev libxcb1-dev libxrandr-dev libxinerama-dev libxcursor-dev libxi-dev \
  libgl1-mesa-dev libasound2-dev
```

Se o linker pedir outra biblioteca (ex.: Wayland), instale o pacote `-dev` correspondente indicado na mensagem de erro.

### Pacotes de runtime: RetroArch e libretro (instalacao manual)

O launcher chama o executavel `retroarch` e cores `*_libretro.so` instalados no sistema. Exemplo de instalacao no Debian/Ubuntu/Raspberry Pi OS:

```bash
sudo apt update
sudo apt install -y retroarch
sudo apt install -y libretro-snes9x libretro-nestopia libretro-mgba
```

Confira nomes disponiveis na sua versao:

```bash
apt search libretro-
apt search '^retroarch'
```

Se um pacote nao existir com esse nome exato, use o pacote sugerido pelo `apt` ou instale o core pelo proprio RetroArch (Online Updater), e aponte `cores_dir` / `default_core` na configuracao do launcher.

### Resumo

| Tipo | Instalado pelo `cargo build`? |
|------|--------------------------------|
| Crates Rust (crates.io) | **Sim** (baixa/compila) |
| SQLite C (via `rusqlite` bundled) | **Compilado dentro do build** (precisa de toolchain C no sistema) |
| Pacotes `.deb` / `apt` (RetroArch, libretro, `-dev`) | **Nao** — instale manualmente (ou via script/CI seu) |

## Fluxo MVP implementado

- escanear ROMs em subpastas por sistema: `library.roms_dir/<sistema>/` (nome da chave em `systems`, ex. `nes`, `snes`);
- BIOS por sistema em `library.bios_dir/<sistema>/`, passada ao RetroArch como `system_directory` via `--appendconfig`;
- validar extensao do ficheiro face a `systems.<sistema>.accepted_extensions` na pasta desse sistema;
- resolver core com `systems.<plataforma>.default_core`;
- executar RetroArch com `--appendconfig` (BIOS), `-L <core> <rom>`;
- persistir `history.last_game_path` ao finalizar o jogo.

## Fase 2 implementada

- scanner recursivo e incremental sincronizado com SQLite;
- banco local em `~/.local/share/rpi_open_emulator/catalog.sqlite3`;
- tela com secoes de biblioteca, recentes e favoritos;
- acoes de favoritar/desfavoritar persistidas no catalogo;
- contagem de partidas (`play_count`) e ultimo jogado (`last_played_at`).
- metadata offline-first no SQLite (`game_metadata`);
- descoberta local de capa por arquivo vizinho e pasta `covers/`;
- botao para atualizar metadata offline sem depender de internet.
- painel de configuracoes na UI para editar RetroArch, ROM/BIOS e mapeamento por sistema.

## Integracao com o desktop Linux

Arquivos em `packaging/`:

- `rpi_open_emulator.desktop.in` — modelo de entrada de aplicativo (`Exec=@EXEC@`).
- `rpi_open_emulator.svg` — icone escalavel (tema Freedesktop).
- `install-desktop.sh` — instala em `~/.local/share/applications/` e `~/.local/share/icons/hicolor/scalable/apps/`.

Depois de compilar o binario:

```bash
cd rpi_open_emulator
cargo build --release
./packaging/install-desktop.sh
```

Opcionalmente passe o caminho do binario:

```bash
./packaging/install-desktop.sh /caminho/completo/rpi_open_emulator
```

Ou defina `RPI_OPEN_EMULATOR_BIN` antes de rodar o script (compativel com `RPI5_LAUNCHER_BIN` legado).

No menu de aplicativos procure por **RPI Open Emulator** (em alguns ambientes e necessario atualizar o cache de icones ou relogar).
