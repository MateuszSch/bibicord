# dmgbuild settings. Paths come from -D defines passed by scripts/build-macos-dmg.sh.
import os

app = defines["app"]  # noqa: F821
bg = defines["background"]  # noqa: F821

format = "ULMO"  # LZMA, macOS 10.15+
filesystem = "APFS"
files = [app]
symlinks = {"Aplikacje": "/Applications"}
icon = defines["volicon"]  # noqa: F821
badge_icon = None
icon_locations = {os.path.basename(app): (170, 200), "Aplikacje": (470, 200)}
background = bg
window_rect = ((200, 160), (640, 400))
default_view = "icon-view"
show_status_bar = False
show_tab_view = False
show_toolbar = False
show_pathbar = False
show_sidebar = False
show_icon_preview = False
arrange_by = None
icon_size = 112
text_size = 13
