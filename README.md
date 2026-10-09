# 🪟 ToolsBoXApp

**Application Windows qui affiche le contenu de [`ToolsBoXMachine`](https://github.com/RackRunner1/ToolsBoXMachine) en local.**

À minuit UTC, une GitHub Action copie le code du site depuis ToolsBoXMachine dans le dossier `contents/` de ce dépôt, recompile l'application, puis publie une **release `YYYYMMDD`** contenant :

* `ToolsBoXApp_x64.msix` — le package de l'app (contenu inclus),
* `ToolsBoXApp.appinstaller` — le fichier d'installation / de mises à jour automatiques,
* `ToolsBoXApp.cer` — le certificat de signature (à installer une fois),
* `install-app.ps1` — installe tout en une commande.

Quand on ouvre l'app : le panneau de gauche liste l'arborescence de `contents/` (dossiers `.assets`, `pages`, `public`, `tools/…`, `ui/`, etc.), et un clic sur un `.html` l'affiche **tel quel** dans un navigateur embarqué (WebView2). Le site tourne exactement comme sur le web (liens absolus, navbar injectée, JS des outils) grâce à un hôte virtuel local `https://toolbox.local/`.

---

## 🚀 Installation (utilisateur final)

**En une commande** — PowerShell :

```powershell
irm https://github.com/RackRunner1/ToolsBoXApp/releases/latest/download/install-app.ps1 | iex
```

le script demande les droits administrateur, installe le certificat dans *Trusted People*, puis ouvre l'installeur Windows.

**Manuelle**

1. Télécharger `ToolsBoXApp.cer` depuis la [dernière release](https://github.com/RackRunner1/ToolsBoXApp/releases/latest) → double-clic → *Installer le certificat* → **Ordinateur local** → `Trusted People`.
2. Télécharger `ToolsBoXApp.appinstaller` → double-clic → *Installer*.

### Mises à jour

Automatiques : l'app vérifie la dernière release à chaque lancement et propose la mise à jour (`UpdateSettings OnLaunch`), sans bloquer le démarrage. La version du paquet `1.0.AA.JJJ` augmente chaque jour.

### ❓ « Est-ce que Windows va afficher un gros warning comme sur macOS ? »

**Non.** Ce n'est pas le comportement d'un `.exe` téléchargé (page bleue/rouge SmartScreen) ni celui de macOS (Gatekeeper). Pour un MSIX :

* si le certificat n'est **pas** installé → l'installation **échoue** simplement (message d'erreur de confiance), il n'y a pas d'écran d'avertissement effrayant ;
* une fois le `.cer` installé (*Trusted People*, une seule fois) → installation **propre et silencieuse**, aucune alerte.

Un vrai certificat de signature de code (payant, ~ 100-300 €/an) permettrait en plus d'éviter cette installation unique du certificat pour vos utilisateurs, et éviterait aussi l'avertissement SmartScreen si un jour vous distribuez un `.exe`. Pour ce projet, le certificat auto-signé suffit.

---

## 🔄 La synchro quotidienne (minuit UTC)

`.github/workflows/daily-release.yml` :

| Étape | Détail |
|---|---|
| 1. `0 0 * * *` | cron GitHub = **minuit UTC** (= 1h ou 2h du matin heure de Paris). Pour minuit à Paris : `0 22 * * *` en été / `0 23 * * *` en hiver — le cron n'accepte que l'UTC. |
| 2. synchro | `scripts/sync-contents.ps1` clone ToolsBoXMachine et copie **uniquement les fichiers du site** : `.assets/`, `pages/`, `public/`, `tools/`, `ui/`, `index.html`, `style.css`, `llms.txt`, `robots.txt`, `sitemap.xml` (miroir : un fichier supprimé côté source est aussi supprimé ici). Écrit aussi `contents/.sync-info.json` (sha source + date), affiché dans la barre d'état de l'app. |
| 3. commit & push | `chore(contents): synchro ToolsBoXMachine YYYYMMDD` sur `main`. |
| 4. build | `msbuild` du projet WAP → `ToolsBoXApp_x64.msix` (x64, .NET 8 autoportant, WebView2). Version `1.0.AA.JJJ`. Vérifs : signature Authenticode, présence de `contents/index.html` **dans** le package, cohérence de version. |
| 5. .appinstaller | XML généré avec `Uri` pointant sur `releases/latest/download/` (URL stable indépendante du tag du jour). |
| 6. release | `softprops/action-gh-release` crée/met à jour la release **`YYYYMMDD`** avec `make_latest: true` (nécessaire pour que l'URL `…/latest/download/…` serve toujours le build le plus récent → mises à jour auto). |

Le workflow se déclenche aussi sur `pull_request` (build seulement, sans synchro ni release) et manuellement (*Run workflow*, avec cases `do_sync` / `do_release`).

---

## 🔐 Certificat de signature

Un MSIX **doit** être signé. Deux modes :

| Mode | Comportement |
|---|---|
| **Par défaut** (aucun secret) | certificat auto-signé généré à la volée dans la CI ⚠️ — comme il change à chaque build, les mises à jour auto ne sont **pas fiables**. |
| **Recommandé** | certificat stable. Le mien est déjà généré (`certs/ToolsBoXApp.pfx`), il suffit de le déclarer en secrets du repo. |

### Configurer le certificat stable (2 secrets)

Après avoir fait le premier push du dépôt :

```powershell
# 1) base64 du .pfx (déjà généré dans certs\ToolsBoXApp.pfx.b64.txt)
$b64 = Get-Content certs\ToolsBoXApp.pfx.b64.txt -Raw
gh secret set CERT_BASE64   --body $b64          # ou via l'UI GitHub
gh secret set CERT_PASSWORD --body toolboxapp
```

Sans `gh` : *Settings → Secrets and variables → Actions → New repository secret*.
`CERT_BASE64` = contenu de `certs\ToolsBoXApp.pfx.b64.txt` ; `CERT_PASSWORD` = `toolboxapp`.

> Le sujet du certificat (`CN=RackRunner`) doit rester identique au champ `Publisher` de `src/ToolsBoXApp.Package/Package.appxmanifest`. Si vous changez l'un, changez l'autre. Le workflow affiche un avertissement en cas d'incohérence.

---

## 🛠️ Développement local

Prérequis : **.NET 8 SDK** et le runtime **WebView2** (déjà présent sur tous les Windows 10/11 à jour) :

```powershell
winget install Microsoft.DotNet.SDK.8
```

```powershell
# 1) récupérer le contenu du site dans contents/
powershell -ExecutionPolicy Bypass -File .\scripts\sync-contents.ps1

# 2) lancer l'app (résolution auto de contents/ : à côté de l'exe, puis en remontant)
dotnet run --project src\ToolsBoXApp -c Debug
```

Variables utiles : `TOOLSBOX_CONTENT_DIR` pour pointer un dossier `contents` personnalisé ;
`%LOCALAPPDATA%\ToolsBoXApp\contents` fait office d'override utilisateur.

Construire le MSIX en local (nécessite le composant MSIX de Visual Studio) :

```powershell
msbuild src\ToolsBoXApp.Package\ToolsBoXApp.Package.wapproj /p:Configuration=Release /p:Platform=x64 /p:SignCertPath=certs\ToolsBoXApp.pfx /p:SignCertPassword=toolboxapp
```

Sortie : `src\ToolsBoXApp.Package\AppX\ToolsBoXApp_x64.msix`.

---

## 🧩 Architecture

```
ToolsBoXApp/
├── .github/workflows/daily-release.yml   # cron minuit UTC + build + release
├── contents/                             # synchro quotidienne (fichiers du site)
├── scripts/sync-contents.ps1             # copie miroir depuis ToolsBoXMachine
├── install-app.ps1                       # install en 1 commande (cert + appinstaller)
├── certs/                                # cert auto-signé (git-ignoré)
└── src/
    ├── ToolsBoXApp/                      # WPF .NET 8 (WebView2, autoportant, x64)
    │   ├── MainWindow.xaml/.cs           #   barre d'outils + explorateur + WebView2
    │   ├── ContentNode.cs                #   arbre contents/ (paresseux)
    │   └── ContentPaths.cs               #   résolution du dossier contents/
    └── ToolsBoXApp.Package/              # WAP -> MSIX signé + assets
        ├── Package.appxmanifest          #   identité RackRunner.ToolsBoXApp (x64)
        └── Assets/                       #   icônes générées depuis le logo TBXM
```

**Fonctionnement interne du contenu** : `CoreWebView2.SetVirtualHostNameToFolderMapping` sert `contents/` sous `https://toolbox.local/`. Les pages utilisent des URLs absolues (`/tools/calculator/calculator.html`, `fetch('/ui/navbar.html')`) : elles fonctionnent donc **sans aucune modification** du site d'origine.

## 📄 Licence

Le code de l'app est sous [Unlicense](LICENSE) — comme ToolsBoXMachine.
