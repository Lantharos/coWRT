# Runs build tools in the toolbox named by COWRT_TOOLBOX (when set) and container tools on the host.

container_name() {
	sed -n 's/^name="\(.*\)"$/\1/p' /run/.containerenv 2>/dev/null
}

on_host() {
	if [ -z "$(container_name)" ] || command -v "$1" >/dev/null; then
		"$@"
	else
		flatpak-spawn --host "$@"
	fi
}

build() {
	if [ -z "${COWRT_TOOLBOX:-}" ] || [ "$(container_name)" = "$COWRT_TOOLBOX" ]; then
		"$@"
		return
	fi
	local run=(toolbox run -c "$COWRT_TOOLBOX" bash -c 'cd "$1" && shift && exec "$@"' _ "$PWD" "$@")
	if [ -z "$(container_name)" ]; then
		"${run[@]}"
	else
		flatpak-spawn --host "${run[@]}"
	fi
}
