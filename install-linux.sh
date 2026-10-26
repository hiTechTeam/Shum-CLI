#!/bin/sh
# Install the latest Linux release for the current user, x64 or ARM64.
set -eu
umask 077

project_url=https://github.com/hiTechTeam/Shum-CLI
release_url=https://github.com/hiTechTeam/Shum-CLI/releases/latest/download
fail() { printf '%s\n' "Shum: $*" >&2; exit 1; }
case ${1-} in
    '') operation=install ;;
    --uninstall) operation=uninstall ;;
    *) fail 'Использование: sh install-linux.sh [--uninstall]' ;;
esac
[ "$#" -le 1 ] || fail 'Слишком много аргументов'
[ "$(uname -s)" = Linux ] || fail "Этот установщик для Linux. Информация: $project_url"
case $(uname -m) in
    x86_64|amd64) arch=x64 ;;
    aarch64|arm64) arch=arm64 ;;
    *) fail "Архитектура не поддерживается. Информация: $project_url" ;;
esac
case ${HOME-} in /*) ;; *) fail 'Домашний каталог недоступен' ;; esac
asset=shum-linux-$arch.tar.gz
# Profiles live in ~/.local/share/shum, so the program stays apart from them.
install_dir=$HOME/.local/lib/shum-cli
bin_dir=$HOME/.local/bin
binary=$install_dir/current/shum
marker=$install_dir/.shum-installer
lock=$HOME/.local/lib/.shum-install.lock
temporary=
cleanup() {
    [ -z "$temporary" ] || rm -rf "$temporary"
    rmdir "$lock" 2>/dev/null || :
}
# Refuse to overwrite another installation or unrelated files.
check_owned() {
    if [ -e "$install_dir" ] || [ -L "$install_dir" ]; then
        [ -d "$install_dir" ] && [ ! -L "$install_dir" ] && [ -f "$marker" ] && [ ! -L "$marker" ] \
            && [ "$(cat "$marker")" = shum-install-v1 ] || fail "Каталог $install_dir не принадлежит этому установщику"
        [ ! -L "$install_dir/versions" ] || fail 'Каталог версий не должен быть ссылкой'
    fi
    if [ -e "$bin_dir/shum" ] || [ -L "$bin_dir/shum" ]; then
        [ -L "$bin_dir/shum" ] && [ "$(readlink "$bin_dir/shum")" = "$binary" ] \
            || fail "Команда $bin_dir/shum уже принадлежит другой установке"
    fi
}
mkdir -p "$HOME/.local/lib"
mkdir "$lock" 2>/dev/null || fail 'Другой установщик работает. После аварийного завершения удалите ~/.local/lib/.shum-install.lock вручную.'
trap cleanup 0
trap 'exit 1' HUP INT TERM
check_owned
if [ "$operation" = uninstall ]; then
    if [ -e "$install_dir" ]; then
        [ -x "$binary" ] || fail 'Бинарник установки отсутствует; сначала остановите службы shum-*.service вручную'
        "$binary" daemon --uninstall
        [ ! -L "$bin_dir/shum" ] || rm "$bin_dir/shum"
        rm -rf "$install_dir"
    elif [ -L "$bin_dir/shum" ]; then
        fail 'Программа отсутствует; остановите службы shum-*.service вручную, затем удалите ссылку ~/.local/bin/shum'
    fi
    printf '%s\n' 'Shum удалён. Данные профилей сохранены:' "${SHUM_DATA_DIR:-${XDG_DATA_HOME:-$HOME/.local/share}/shum}" \
        'Другие каталоги --data-dir и ключи профилей в хранилище секретов также сохранены.'
    exit 0
fi
for tool in curl sha256sum tar; do
    command -v "$tool" >/dev/null 2>&1 || fail "Не найдена системная команда $tool"
done
temporary=$(mktemp -d "${TMPDIR:-/tmp}/shum-install.XXXXXXXX")
curl -fSL --proto '=https' --tlsv1.2 "$release_url/$asset" -o "$temporary/$asset"
curl -fSL --proto '=https' --tlsv1.2 "$release_url/$asset.sha256" -o "$temporary/checksum"
expected=$(awk -v name="$asset" 'NF == 2 && $2 == name { print $1; found++ } END { if (NR != 1 || found != 1) exit 1 }' "$temporary/checksum") \
    || fail 'Недействительный файл SHA-256'
case $expected in *[!0-9a-fA-F]*|'') fail 'Недействительный SHA-256' ;; esac
[ "${#expected}" -eq 64 ] || fail 'Недействительная длина SHA-256'
actual=$(sha256sum "$temporary/$asset")
actual=${actual%% *}
[ "$actual" = "$(printf '%s' "$expected" | tr 'A-F' 'a-f')" ] || fail 'SHA-256 не совпадает. Установка отменена.'
# The published archive has precisely these two files, with no links or paths.
printf '%s\n' LICENSE shum > "$temporary/expected"
tar -tzf "$temporary/$asset" | LC_ALL=C sort > "$temporary/sorted" || fail 'Архив повреждён'
cmp -s "$temporary/expected" "$temporary/sorted" || fail 'Неожиданное содержимое архива'
tar -tvzf "$temporary/$asset" > "$temporary/types" || fail 'Архив повреждён'
awk 'substr($0,1,1) != "-" { exit 1 }' "$temporary/types" || fail 'Ссылки и специальные файлы в архиве запрещены'
mkdir "$temporary/unpacked"
tar -xzf "$temporary/$asset" -C "$temporary/unpacked"
chmod 755 "$temporary/unpacked/shum"
version=$("$temporary/unpacked/shum" --version 2>/dev/null) || fail 'Программа не запускается на этой системе. Нужны glibc 2.35 или новее (Ubuntu 22.04, Debian 12).'
case $version in 'shum '*) version=${version#shum } ;; *) fail 'Неожиданная версия программы' ;; esac
mkdir -p "$install_dir/versions" "$bin_dir"
printf '%s\n' shum-install-v1 > "$marker"
destination=$install_dir/versions/$actual
if [ -e "$destination" ] || [ -L "$destination" ]; then
    [ -d "$destination" ] && [ ! -L "$destination" ] || fail 'Недействительный каталог версии'
    cmp -s "$destination/shum" "$temporary/unpacked/shum" || fail 'Существующая версия отличается от проверенного архива'
else
    mkdir "$destination"
    mv "$temporary/unpacked/shum" "$temporary/unpacked/LICENSE" "$destination/"
fi
[ ! -e "$install_dir/current" ] || [ -L "$install_dir/current" ] || fail 'current должен быть ссылкой установщика'
ln -s "versions/$actual" "$install_dir/.next"
# mv -T atomically replaces the link instead of moving into the directory it points to.
mv -fT "$install_dir/.next" "$install_dir/current"
if [ ! -L "$bin_dir/shum" ]; then ln -s "$binary" "$bin_dir/shum"; fi
# Refresh active profiles; first install never opens keys or creates a profile.
"$binary" daemon --refresh
printf '%s\n' "Shum $version установлен. Запуск: shum"
case :${PATH-}: in
    *:"$bin_dir":*) ;;
    *) printf '%s\n' 'Добавьте ~/.local/bin в PATH командой:' "export PATH=\"\$HOME/.local/bin:\$PATH\"" \
        'Файлы настройки оболочки не изменены.' ;;
esac
