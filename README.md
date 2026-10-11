# ToolsBoXApp

<img width="1519" height="880" alt="image" src="https://github.com/user-attachments/assets/8fb705e8-1c4e-4027-970c-7914f792c942" />

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
- 🔜 Arch Linux support is coming next week

## Installation

### Option 1 — Automatic (recommended)

Open a terminal and run:

```sh
curl -fsSL https://tbxm.org/install.sh | bash
```

The script detects your distribution and installs the right package for you.

### Option 2 — Manual

1. Download the package for your distribution from the
   [latest release](https://github.com/RackRunner1/ToolsBoXApp/releases/latest):
   the `.deb` file for Debian/Ubuntu, the `.rpm` file for Fedora/Red Hat.
2. Install it:
   - Debian/Ubuntu: `sudo apt install ./<file>.deb`
   - Fedora/Red Hat: `sudo dnf install ./<file>.rpm`

## Update & uninstall

- Update: `tbxa update`
- Uninstall: `tbxa uninstall`
