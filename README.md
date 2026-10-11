# handy-groq

**Handy fork that uses Groq for instant transcription, built in Rust**

efficient rust-based transcription tool, uses groq, BYOK

![Handy-fork](https://img.shields.io/badge/fork-Handy%20fork-8A2BE2)

this is a fork of cjpais/Handy, rebuilt from the ground up in pure rust. no
tauri, no webview/webkit, no local models chewing cpu on your machine. it's just
a tiny native window (eframe/egui) that records your mic, ships the audio to
groq's whisper api, and pastes the text right where your cursor sits. u bring
the api key (BYOK), nothing else to install — dirt simple, dig it.

## what it does
- hold a global shortcut to record, release to transcribe
- text comes back from groq and gets pasted auto (or just copied to the
  clipboard if u prefer)
- one little settings window, saved to a single json file

## how to use (3 steps)
1. **paste your groq api key** — grab one at [console.groq.com](https://console.groq.com),
   drop it in the "groq api key" box, hit 💾 save.
2. **hold the shortcut** — default `ctrl+shift+space`. talk while u hold it.
3. **release** — sends audio to groq, transcribes it, pastes the text into
   whatever app u're typing in. done, khalas.

## what you need
- a groq api key (free tier works). that's it. no accounts beyond groq, no local
  llms to download.
- windows, macos, linux (intel + arm).

## keyboard shortcut
default: `ctrl+shift+space` (press + hold to record, release to stop).

change it in settings — like `ctrl+shift+space` or `cmd+shift+x` on mac. it
takes effect after u save. ya, groq sees this as the trigger too, so if u want a
different combo pick something not used by another app.

## where settings are saved
one json file, no registry junk, no roaming config. the app drops it in ur
config dir:

- linux: `~/.config/handy-groq/settings.json`
- macos: `~/Library/Application Support/handy-groq/settings.json`
- windows: `%APPDATA%\handy-groq/settings.json`

ur api key lives in there as plain text — fine cz it stays on ur machine and
groq's api is the only thing that ever hears ur audio.

## privacy
ur mic audio only ever goes to the groq api over https. nothing stored locally,
nothing cached, nothing logged to disk. the api key sits in that one settings
file on ur computer and nowhere else. once transcription comes back, the audio's
gone.

## builds
all releases built on github actions (see build.yml). no code signing, so
download the zip, extract, run `handy-groq`. on macos u'll need to
right-click-open the first time since it's unsigned.

built with: eframe 0.36, cpal, hound, reqwest + tokio, global-hotkey 0.8,
enigo 0.6, arboard 3.

built in rust. no webview. no local models. just u, ur mic, and groq.
