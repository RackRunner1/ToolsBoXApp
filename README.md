# ToolsBoXApp
A project by the ToolsBoXCompany.

---

<img width="1490" height="888" alt="image" src="https://github.com/user-attachments/assets/1d64c11e-c446-4454-87cf-04abb632b2b9" />

---

ToolsBoXApp brings **ToolsBoXMachine** to your desktop as a real app.
It runs **locally on your computer**, so once it is installed you can use
all your tools **without an internet connection**.

The content is **refreshed every day** from ToolsBoXMachine, so you always
have the latest version of every tool. Just run `tbxa update` and you are
up to date.

## Compatibility

- ✅ Debian, Ubuntu and derivatives (`.deb` package)
- ✅ Fedora, Red Hat and derivatives (`.rpm` package)
- ✅ Arch Linux and derivatives (`.pkg.tar.zst` package & AUR `toolsboxapp-bin`)
- ✅ Universal Linux via Flatpak (Flathub `io.github.RackRunner1.ToolsBoXApp`)

## Installation

### Option 1: Automatic (recommended)

Open a terminal and run:

```sh
curl -fsSL https://tbxm.org/install.sh | bash
```

The script detects your distribution and installs the right package for you.

### Option 2: Flatpak (Flathub)

```sh
flatpak install flathub io.github.RackRunner1.ToolsBoXApp
flatpak run io.github.RackRunner1.ToolsBoXApp
```

### Option 3: Arch Linux (AUR)

```sh
yay -S toolsboxapp-bin
# or
paru -S toolsboxapp-bin
```

### Option 4: Manual

1. Download the package for your distribution from the
   [latest release](https://github.com/RackRunner1/ToolsBoXApp/releases/latest):
   the `.deb` file for Debian/Ubuntu, the `.rpm` file for Fedora/Red Hat,
   the `.pkg.tar.zst` file for Arch Linux.
2. Install it:
   - Debian/Ubuntu: `sudo apt install ./<file>.deb`
   - Fedora/Red Hat: `sudo dnf install ./<file>.rpm`
   - Arch Linux: `sudo pacman -U ./<file>.pkg.tar.zst`

## Update & uninstall

`tbxa` and `tbxm` are the same command (alias):

- Update: `tbxa update` (or `tbxm update`)
- Uninstall: `tbxa uninstall` (or `tbxm uninstall`)

