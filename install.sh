#!/bin/sh
# Install the signed universal release for the current macOS user.
set -eu
umask 077

project_url=https://github.com/hiTechTeam/Shum-CLI
release_url=https://github.com/hiTechTeam/homebrew-shum/releases/latest/download
asset=shum-macos-universal.tar.gz
certificate=9ce51b5c3430742e7860fc77583b41baa8355085
fail() { printf '%s\n' "Shum: $*" >&2; exit 1; }
case ${1-} in
    '') operation=install ;;
    --uninstall) operation=uninstall ;;
    *) fail 'Использование: sh install.sh [--uninstall]' ;;
esac
[ "$#" -le 1 ] || fail 'Слишком много аргументов'
[ "$(uname -s)" = Darwin ] || fail "Этот установщик поддерживает macOS 15+, Apple Silicon и Intel. Информация: $project_url"
case $(uname -m) in arm64|x86_64) ;; *) fail "Архитектура не поддерживается. Информация: $project_url" ;; esac
case ${HOME-} in /*) ;; *) fail 'Домашний каталог недоступен' ;; esac
install_dir=$HOME/.local/share/shum
bin_dir=$HOME/.local/bin
binary=$install_dir/Shum.app/Contents/MacOS/shum
marker=$install_dir/.shum-installer
lock=$HOME/.local/share/.shum-install.lock
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
mkdir -p "$HOME/.local/share"
mkdir "$lock" 2>/dev/null || fail 'Другой установщик работает. После аварийного завершения удалите ~/.local/share/.shum-install.lock вручную.'
trap cleanup 0
trap 'exit 1' HUP INT TERM
check_owned
if [ "$operation" = uninstall ]; then
    if [ -e "$install_dir" ]; then
        [ -x "$binary" ] || fail 'Бинарник установки отсутствует; сначала очистите LaunchAgents вручную по README'
        "$binary" daemon --uninstall
        [ ! -L "$bin_dir/shum" ] || rm "$bin_dir/shum"
        rm -rf "$install_dir"
    elif [ -L "$bin_dir/shum" ]; then
        fail 'Бандл отсутствует; очистите LaunchAgents вручную по README, затем удалите ссылку ~/.local/bin/shum'
    fi
    printf '%s\n' 'Shum удалён. Данные профилей сохранены:' "${SHUM_DATA_DIR:-$HOME/Library/Application Support/org.Shum.Shum}" \
        'Другие каталоги --data-dir и ключи профилей в Keychain также сохранены.'
    exit 0
fi
# Do not install a second copy alongside Homebrew, including when brew is absent from PATH.
if [ -d /opt/homebrew/opt/shum ] || [ -d /usr/local/opt/shum ]; then
    fail 'Shum уже установлен через Homebrew. Используйте brew upgrade shum.'
fi
if command -v brew >/dev/null 2>&1 && HOMEBREW_NO_AUTO_UPDATE=1 brew list --versions shum >/dev/null 2>&1; then
    fail 'Shum уже установлен через Homebrew. Используйте brew upgrade shum.'
fi
system_version=$(sw_vers -productVersion)
major=${system_version%%.*}
case $major in ''|*[!0-9]*) fail 'Не удалось определить версию macOS' ;; esac
[ "$major" -ge 15 ] || fail 'Нужна macOS 15 или новее'
for tool in curl shasum tar codesign plutil; do
    command -v "$tool" >/dev/null 2>&1 || fail "Не найдена системная команда $tool"
done
temporary=$(mktemp -d "${TMPDIR:-/tmp}/shum-install.XXXXXXXX")
curl -fSL --proto '=https' --tlsv1.2 "$release_url/$asset" -o "$temporary/$asset"
curl -fSL --proto '=https' --tlsv1.2 "$release_url/$asset.sha256" -o "$temporary/checksum"
expected=$(awk -v name="$asset" 'NF == 2 && $2 == name { print $1; found++ } END { if (NR != 1 || found != 1) exit 1 }' "$temporary/checksum") \
    || fail 'Недействительный файл SHA-256'
case $expected in *[!0-9a-fA-F]*|'') fail 'Недействительный SHA-256' ;; esac
[ "${#expected}" -eq 64 ] || fail 'Недействительная длина SHA-256'
actual=$(shasum -a 256 "$temporary/$asset")
actual=${actual%% *}
[ "$actual" = "$(printf '%s' "$expected" | tr 'A-F' 'a-f')" ] || fail 'SHA-256 не совпадает. Установка отменена.'
# The published bundle has precisely these entries, with no links or traversal paths.
cat > "$temporary/expected" <<'FILES'
Shum.app
Shum.app/Contents
Shum.app/Contents/Info.plist
Shum.app/Contents/MacOS
Shum.app/Contents/MacOS/shum
Shum.app/Contents/_CodeSignature
Shum.app/Contents/_CodeSignature/CodeResources
FILES
tar -tzf "$temporary/$asset" > "$temporary/list" || fail 'Архив повреждён'
sed 's:/$::' "$temporary/list" | LC_ALL=C sort > "$temporary/sorted"
cmp -s "$temporary/expected" "$temporary/sorted" || fail 'Неожиданное содержимое архива'
tar -tvzf "$temporary/$asset" > "$temporary/types" || fail 'Архив повреждён'
awk 'substr($0,1,1) != "-" && substr($0,1,1) != "d" { exit 1 }' "$temporary/types" \
    || fail 'Ссылки и специальные файлы в архиве запрещены'
mkdir "$temporary/unpacked"
tar -xzf "$temporary/$asset" -C "$temporary/unpacked"
app=$temporary/unpacked/Shum.app
codesign --verify --strict --all-architectures \
    -R "=identifier \"org.shum.cli\" and certificate leaf = H\"$certificate\"" "$app" \
    || fail 'Подпись Shum.app не прошла проверку. Настройки доверия macOS не изменены.'
for architecture in arm64 x86_64; do
    codesign --display --arch "$architecture" "$app" >/dev/null 2>&1 || fail 'Архив не содержит обе архитектуры'
done
version=$(plutil -extract CFBundleShortVersionString raw -o - "$app/Contents/Info.plist")
[ "$("$app/Contents/MacOS/shum" --version)" = "shum $version" ] || fail 'Версия бинарника не совпадает с бандлом'
mkdir -p "$install_dir/versions" "$bin_dir"
printf '%s\n' shum-install-v1 > "$marker"
destination=$install_dir/versions/$actual
if [ -e "$destination" ] || [ -L "$destination" ]; then
    [ -d "$destination" ] && [ ! -L "$destination" ] || fail 'Недействительный каталог версии'
    codesign --verify --strict --all-architectures \
        -R "=identifier \"org.shum.cli\" and certificate leaf = H\"$certificate\"" "$destination/Shum.app" \
        || fail 'Существующий бандл повреждён'
    cmp -s "$destination/Shum.app/Contents/MacOS/shum" "$app/Contents/MacOS/shum" || fail 'Существующая версия отличается от проверенного архива'
    cmp -s "$destination/Shum.app/Contents/Info.plist" "$app/Contents/Info.plist" || fail 'Метаданные существующего бандла отличаются'
    cmp -s "$destination/Shum.app/Contents/_CodeSignature/CodeResources" "$app/Contents/_CodeSignature/CodeResources" || fail 'Ресурсы подписи существующего бандла отличаются'
else
    mkdir "$destination"
    mv "$app" "$destination/Shum.app"
fi
[ ! -e "$install_dir/Shum.app" ] || [ -L "$install_dir/Shum.app" ] || fail 'Shum.app должен быть ссылкой установщика'
ln -s "versions/$actual/Shum.app" "$install_dir/.next-app"
# macOS mv -h atomically replaces a directory symlink instead of following it.
/bin/mv -fh "$install_dir/.next-app" "$install_dir/Shum.app"
if [ ! -L "$bin_dir/shum" ]; then ln -s "$binary" "$bin_dir/shum"; fi
# Refresh active profiles; first install never opens keys or creates a profile.
"$binary" daemon --refresh
printf '%s\n' "Shum $version установлен. Запуск: shum"
case :${PATH-}: in
    *:"$bin_dir":*) ;;
    *) printf '%s\n' 'Добавьте ~/.local/bin в PATH командой:' "export PATH=\"\$HOME/.local/bin:\$PATH\"" \
        'Файлы настройки оболочки не изменены.' ;;
esac
