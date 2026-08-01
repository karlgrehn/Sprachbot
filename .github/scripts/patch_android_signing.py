#!/usr/bin/env python3
"""Fügt die Release-Signing-Konfiguration in das von `tauri android init`
generierte app/build.gradle.kts ein — laut offizieller Tauri-v2-Doku
(distribute/sign/android) ist das kein CLI-Schalter, sondern eine manuelle
Bearbeitung dieser Datei, die hier automatisiert per CI läuft, statt sie
von Hand im generierten (nicht eingecheckten) Projekt zu pflegen.
"""
import re
import sys

# NUR FileInputStream importieren, nicht zusaetzlich java.util.Properties:
# Run #15 zeigte "Conflicting import, imported name 'Properties' is
# ambiguous" - Gradles Kotlin-DSL spleisst Properties offenbar bereits
# implizit in jedes Build-Skript ein, ein zusaetzlicher expliziter Import
# davon kollidiert damit. Die offizielle Tauri-Doku importiert deshalb auch
# nur FileInputStream, nicht Properties - exakt danach richten, nicht
# "vorsichtshalber" ergaenzen.
IMPORTS = "import java.io.FileInputStream\n"

SIGNING_CONFIG = """    signingConfigs {
        create("release") {
            val keystorePropertiesFile = rootProject.file("keystore.properties")
            val keystoreProperties = Properties()
            if (keystorePropertiesFile.exists()) {
                keystoreProperties.load(FileInputStream(keystorePropertiesFile))
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

    # Import an den Dateianfang statt java.util.Properties()/java.io.Xxx()
    # inline zu qualifizieren: Run #14 zeigte den echten Grund dafür ("error:
    # Unresolved reference: util"/"io") - innerhalb des generierten
    # android{}-Scopes ist der Bezeichner "java" selbst mehrdeutig (vermutlich
    # durch eine Gradle/AGP-DSL-Erweiterung überschattet), sodass
    # "java.util.Properties()" dort nicht auf das Top-Level-Package
    # aufgelöst wird. Import + unqualifizierter Klassenname (exakt wie in
    # der offiziellen Tauri-Doku) umgeht das.
    if "import java.io.FileInputStream" in content:
        print(f"{path}: Imports bereits vorhanden, überspringe.")
    else:
        content = IMPORTS + content

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
