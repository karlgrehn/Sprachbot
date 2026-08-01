#!/usr/bin/env python3
"""Fügt die Release-Signing-Konfiguration in das von `tauri android init`
generierte app/build.gradle.kts ein — laut offizieller Tauri-v2-Doku
(distribute/sign/android) ist das kein CLI-Schalter, sondern eine manuelle
Bearbeitung dieser Datei, die hier automatisiert per CI läuft, statt sie
von Hand im generierten (nicht eingecheckten) Projekt zu pflegen.
"""
import re
import sys

SIGNING_CONFIG = """    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = java.util.Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(java.io.FileInputStream(keystorePropertiesFile))
            }

            keyAlias = keystoreProperties["keyAlias"] as String
            keyPassword = keystoreProperties["password"] as String
            storeFile = file(keystoreProperties["storeFile"] as String)
            storePassword = keystoreProperties["password"] as String
        }
    }

"""


def main():
    if len(sys.argv) != 2:
        print("Verwendung: patch_android_signing.py <pfad zu app/build.gradle.kts>", file=sys.stderr)
        sys.exit(1)

    path = sys.argv[1]
    with open(path, "r", encoding="utf-8") as f:
        content = f.read()

    if "signingConfigs" in content:
        print(f"{path}: signingConfigs bereits vorhanden, überspringe Einfügen.")
    else:
        match = re.search(r"^(\s*)buildTypes\s*\{", content, re.MULTILINE)
        if not match:
            print(f"FEHLER: kein 'buildTypes {{' in {path} gefunden — Vorlage hat sich geändert, Skript anpassen.", file=sys.stderr)
            print("--- Dateiinhalt ---", file=sys.stderr)
            print(content, file=sys.stderr)
            sys.exit(1)
        insert_at = match.start()
        content = content[:insert_at] + SIGNING_CONFIG + content[insert_at:]

    if re.search(r'getByName\("release"\)\s*\{\s*\n\s*signingConfig\s*=', content):
        print(f"{path}: signingConfig-Zuweisung bereits vorhanden, überspringe.")
    else:
        release_match = re.search(r'(getByName\("release"\)\s*\{)', content)
        if not release_match:
            print(f"FEHLER: kein 'getByName(\"release\") {{'-Block in {path} gefunden.", file=sys.stderr)
            print("--- Dateiinhalt ---", file=sys.stderr)
            print(content, file=sys.stderr)
            sys.exit(1)
        insert_at = release_match.end()
        content = (
            content[:insert_at]
            + '\n            signingConfig = signingConfigs.getByName("release")'
            + content[insert_at:]
        )

    with open(path, "w", encoding="utf-8") as f:
        f.write(content)

    print(f"{path}: Signing-Konfiguration eingefügt.")


if __name__ == "__main__":
    main()
