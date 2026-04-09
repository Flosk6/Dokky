# Dokky

Multi-compte Dofus Touch sur PC. Jouez plusieurs comptes simultanément depuis une seule application.

<p align="center">
  <img src="src/assets/dokky-logo.png" alt="Dokky" width="128" />
</p>

## Fonctionnalités

- **Multi-instance** : Ouvrez plusieurs instances de Dofus Touch sur un même device Android
- **Interface à onglets** : Naviguez entre vos comptes comme des onglets Chrome
- **Vidéo intégrée** : Rendu H.264 directement dans l'app via WebCodecs (pas de fenêtre scrcpy)
- **Clonage APK** : Créez des clones de Dofus Touch avec noms et icônes personnalisés
- **Raccourcis clavier** : Mappez des touches à des zones de l'écran (sorts, actions) avec support appui long *
- **Presets de performance** : Ultra, High, Medium, Low ou configuration personnalisée
- **Multi-device** : Gérez plusieurs téléphones Android simultanément *

\* *Fonctionnalité Pro*

## Pré-requis

- Un téléphone Android avec **Dofus Touch** installé
- **USB Debugging** activé sur le téléphone ([comment faire](https://developer.android.com/studio/debug/dev-options))
- Un câble USB

## Installation

### macOS

1. Téléchargez `Dokky-x.x.x-macos.zip` depuis les [Releases](../../releases)
2. Décompressez et glissez `Dokky.app` dans `/Applications`
3. Au premier lancement : clic droit > Ouvrir (pour contourner Gatekeeper)

### Windows

1. Téléchargez `Dokky-x.x.x-windows.msi` depuis les [Releases](../../releases)
2. Lancez l'installeur

## Utilisation

1. **Branchez** votre téléphone Android en USB
2. **Acceptez** le popup "Autoriser le débogage USB" sur le téléphone
3. Votre device apparaît dans Dokky
4. Cliquez **Nouvelle instance** (ou `Ctrl+T`)
5. Sélectionnez le compte Dofus Touch à lancer
6. Jouez !

### Raccourcis de navigation

| Raccourci | Action |
|-----------|--------|
| `Ctrl+T` | Nouvelle instance |
| `Ctrl+W` | Fermer l'instance active |
| `Ctrl+1-9` | Aller à l'onglet N |
| `Ctrl+Tab` | Onglet suivant |
| `Ctrl+Shift+Tab` | Onglet précédent |

### Clonage de comptes

Pour jouer plusieurs comptes, vous devez créer des **clones** de Dofus Touch :

1. Ouvrez le panneau **Devices** (icône téléphone dans la sidebar)
2. Cliquez **+ Nouveau clone APK**
3. Donnez un nom (ex: "Féca PvP") et une couleur
4. Attendez le clonage (~30 secondes)
5. Le clone apparaît dans la liste et peut être lancé comme une instance séparée

### Raccourcis en jeu (Pro)

1. Cliquez sur l'icône **touche clavier** dans la sidebar
2. Cliquez sur une zone vide ou dessinez une zone sur l'écran
3. Assignez une touche et un label
4. En jeu, appuyez sur la touche pour simuler un tap dans la zone
5. Maintenez la touche pour un appui long

## Performance

Ouvrez le panneau **Performance** (icône sliders) pour ajuster :

- **Presets** : Ultra (4K/60fps), High (1080p/60fps), Medium (720p/45fps), Low (540p/30fps)
- **Custom** : résolution, DPI, FPS, bitrate, i-frame interval
- **Optimisations device** : désactiver les animations Android, luminosité minimale

> **Conseil** : Pour 3+ instances, utilisez le preset Medium ou Low. 30 FPS suffit largement pour Dofus Touch.

## Licence Pro

Dokky est gratuit pour jouer en multi-instance sur un seul device. La licence Pro débloque :

- Multi-device (plusieurs téléphones)
- Raccourcis clavier

Prix : 2€/mois ou 20€/an

## Développement

```bash
# Pré-requis : Node.js 20+, Rust 1.77+, adb, scrcpy

# Installer les dépendances
npm install

# Collecter les deps externes (adb, scrcpy-server, apktool, JRE)
bash scripts/collect-deps.sh

# Lancer en mode dev
cargo tauri dev

# Build production
cargo tauri build
```

## Stack

- **Backend** : Tauri v2 + Rust
- **Frontend** : Vue 3 + TypeScript + Vite
- **Vidéo** : WebCodecs (H.264) avec transfert binaire direct
- **Protocole** : Communication directe avec scrcpy-server (pas de CLI scrcpy)

## Limitations connues

- `--new-display` n'est pas stable sur tous les devices/ROMs Android
- La saisie de texte dans le jeu (chat) est bufférisée sur les virtual displays Android — le texte apparaît au refocus du champ. C'est une limitation Android, pas un bug Dokky.
- Les performances dépendent du CPU/GPU du téléphone, de la bande passante USB, et du nombre d'instances

## Licence

Ce projet est open source. Voir [LICENSE](LICENSE) pour les détails.
