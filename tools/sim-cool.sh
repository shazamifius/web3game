#!/usr/bin/env bash
# ----------------------------------------------------------------------------
# sim-cool.sh — pousse TOUS les ventilos au MAX le temps d'une commande (une
# simulation), puis REND LA MAIN à la gestion automatique de la carte mère ASUS
# à la sortie (fin normale, Ctrl-C ou kill : la restauration est garantie).
#
# Cible : PC TOUR avec carte mère ASUS (puce capteur Nuvoton nct6775 typique).
# On pilote les ventilos par leur PWM (0–255) exposé dans /sys/class/hwmon.
#
# PRÉ-REQUIS (une seule fois) : la puce ventilo doit être chargée, sinon il n'y
# a AUCUN fichier pwm à régler. Sur NixOS, dans /etc/nixos/configuration.nix :
#     boot.kernelModules = [ "nct6775" ];
#     boot.kernelParams  = [ "acpi_enforce_resources=lax" ];  # souvent requis ASUS
#     environment.systemPackages = [ pkgs.lm_sensors ];
#   puis  sudo nixos-rebuild switch  &&  sudo sensors-detect --auto  && reboot.
#   Vérifie ensuite :  ls /sys/class/hwmon/*/pwm1   (doit lister des fichiers).
#
# USAGE :
#   ./tools/sim-cool.sh <commande…>
# EXEMPLE :
#   ./tools/sim-cool.sh nix-shell --run "cargo run --release -- sim 300 5 20"
#
# Écrire dans /sys exige les droits root → le script utilise `sudo` au besoin
# (il demandera ton mot de passe une fois).
# ----------------------------------------------------------------------------
set -uo pipefail

# Liste des PWM trouvés, et sauvegarde de leur état d'origine (pour restaurer).
PWMS=()
declare -A SAVED_VAL    # pwm → valeur 0–255 d'origine
declare -A SAVED_EN     # pwm → mode d'origine (pwmN_enable)

discover() {
    for p in /sys/class/hwmon/hwmon*/pwm[0-9]; do
        [ -e "$p" ] || continue
        PWMS+=("$p")
        SAVED_VAL["$p"]="$(cat "$p" 2>/dev/null || echo 255)"
        [ -e "${p}_enable" ] && SAVED_EN["$p"]="$(cat "${p}_enable" 2>/dev/null || echo 5)"
    done
}

restore() {
    [ ${#PWMS[@]} -eq 0 ] && return
    echo "[ventilos] restauration de l'auto-ASUS (état d'origine)…"
    for p in "${PWMS[@]}"; do
        # On rend d'abord la valeur, puis le MODE d'origine (souvent 5 = SmartFan
        # auto sur ASUS, ou 2 = thermal-cruise) : la carte mère reprend la main.
        echo "${SAVED_VAL[$p]}" | sudo tee "$p" >/dev/null 2>&1 || true
        if [ -n "${SAVED_EN[$p]:-}" ]; then
            echo "${SAVED_EN[$p]}" | sudo tee "${p}_enable" >/dev/null 2>&1 || true
        fi
    done
}
trap restore EXIT INT TERM

discover

if [ ${#PWMS[@]} -eq 0 ]; then
    echo "[ventilos] ⚠ AUCUN fichier pwm trouvé → pas de contrôle logiciel disponible."
    echo "           La puce ventilo (nct6775) n'est pas chargée. Deux options :"
    echo "           • le plus simple : règle Q-Fan → « Full Speed » dans le BIOS ASUS ;"
    echo "           • ou active nct6775 (voir l'en-tête de ce script), puis relance."
    echo "[ventilos] je lance quand même la commande (sans booster)…"
else
    echo "[ventilos] ${#PWMS[@]} ventilo(s) trouvé(s) → passage au MAX (255) :"
    for p in "${PWMS[@]}"; do
        # 1 = mode manuel ; puis 255 = pleine vitesse.
        [ -e "${p}_enable" ] && echo 1 | sudo tee "${p}_enable" >/dev/null 2>&1 || true
        echo 255 | sudo tee "$p" >/dev/null 2>&1 || true
        echo "           $p → 255 (avant : ${SAVED_VAL[$p]})"
    done
fi

echo "[ventilos] lancement : $*"
echo "----------------------------------------------------------------------"
"$@"
status=$?
echo "----------------------------------------------------------------------"
# restore() est appelé automatiquement par le trap juste après cette ligne.
exit $status
