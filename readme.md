# Sdmap

https://user-images.githubusercontent.com/26996026/217802779-e0e85afe-37fc-4486-a3a4-bf1cb3db23c3.mp4

---

Sdmap remaps the Steam Deck controller and provides a gamepad mode and a desktop mode without the need to launch Steam. You need a kernel with support for the controller (linux >=6.3).

~~~
SD controller -> Sdmap -> ungrab -> game
(input dev)            -> grab -> keyboard+trackpad (virtual dev) -> libinput -> wayland/xorg
~~~

**Please note that since [linux 6.8](https://github.com/torvalds/linux/commit/cd438e57dd05b077f4e87c1567beafb2377b6d6b) the controller starts in lizard mode. Long press the start button to switch to gamepad mode to be able to use sdmap.**

## Install

A [PKGBUILD](arch/) is provided to [build and install](https://wiki.archlinux.org/title/Arch_User_Repository#Installing_and_upgrading_packages) Sdmap on Arch Linux. Once installed, the `sdmap.service` systemd service can be enabled and started (`systemctl enable --now sdmap.service`). The daemon can also be tested outside the service by running `sdmap-daemon`.

You might also want to use sdmap during early userspace to be able to type your encryption passphrase without an external keyboard. To do so, add the `sdmap` hook before `encrypt` in the `HOOKS` array of `/etc/mkinitcpio.conf`.

## Keybindings

* `BTN_THUMB`: switch between gamepad and desktop mode
* gamepad mode (ungrabbed input device)
* desktop mode (grabbed input device & output to virtual device)
    * pointer
        * `ABS_HAT1{X,Y}`: cursor
        * `ABS_{X,Y}`: scroll
        * `BTN_TR`: left click
        * `BTN_TL`: right click
        * `BTN_TL2`: middle click
        * (libinput) middle click + cursor: scroll
    * keyboard
        * `ABS_HAT0{X,Y}` + `BTN_{SOUTH,EAST,NORTH,WEST,START,BASE,THUMBR}`: virtual keyboard
        * `BTN_DPAD_{UP,DOWN,LEFT,RIGHT}`: arrow keys
        * `BTN_TRIGGER_HAPPY2 + BTN_DPAD_{UP,DOWN,LEFT,RIGHT}`: pageup, pagedown, home, end
        * `BTN_SELECT`: tab
        * `BTN_START`: delete
        * `BTN_TRIGGER_HAPPY{1,3,4,2}`: shift, ctrl, alt, altgr
        * `BTN_TR2`: super
        * `BTN_SOUTH`: enter
        * `BTN_EAST`: esc
        * `BTN_NORTH`: backspace
        * `BTN_WEST`: space
        * `BTN_BASE`: compose
    * unused: `BTN_MODE`, `BTN_THUMBL`, `BTN_THUMBR` alone, `BTN_THUMB2`, `ABS_R{X,Y}`, `ABS_HAT2{X,Y}`

## Virtual Keyboard Sticker

![sticker](https://i.imgur.com/DHEOmFD.png)

A [sticker](https://i.imgur.com/DHEOmFD.png) can be generated and [printed](https://i.imgur.com/a7Mk0GY.jpg) for the virtual keyboard on the left trackpad. It's a simple solution that didn't require me to develop a GUI. I printed the sticker on photo paper and glued it on the trackpad. The controller vibrates to provide feedback when the user moves their finger between cells.

~~~sh
sdmap-sticker > sticker.html
chromium --headless --screenshot=sticker.png sticker.html
convert -trim -density 300 sticker.png{,} # 378px / 300ppi = 1.26in ≈ 3.2cm
~~~

## GUI

![sdmap-gui desktop notification](https://i.imgur.com/SkPcML1.png)

I initially didn't develop a GUI and only relied on the sticker described above. Unfortunately I found it frustrating, often mistyping a character from the row above or below the one I intended. The current iteration of `sdmap-gui` is a simple desktop notification sent each time the finger moves to a different cell on the left trackpad.
