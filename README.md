# handy-groq

**Handy fork that uses Groq for instant transcription, built in Rust**

efficient rust-based transcription tool, uses groq, BYOK

---

this is a fork of cjpais/Handy but rebuilt from the ground up in pure rust.
no tauri. no webview/webkit. no local models running on your machine. it's just
a small native window (eframe / egui) that grabs your mic, ships the audio off to
groq's whisper api, and pastes the text right where your cursor is. you bring
the api key (BYOK = bring your own key), there's nothing else to install.

what it does:
- hold a global shortcut to record, release to transcribe
- text comes back from groq and gets pasted automatically (or just copied to
  clipboard if you prefer)
- everything lives in one little settings window, saved to a single json file

## how to use (3 steps)

1. **paste your groq api key** — grab one from [console.groq.com](https://console.groq.com),
   drop it in the "groq api key" box, hit 💾 save.
2. **hold the shortcut** — default is `ctrl+shift+space`. speak while you hold it.
3. **release** — it sends your audio to groq, transcribes it, and pastes the text
   into whatever app you're typing in. done.

## what you need

- a groq api key (free tier works). that's it. no accounts beyond groq, no local
  llms to download.
- works on windows, macos, and linux (intel + arm).

## keyboard shortcut

default: `ctrl+shift+space` (press + hold to record, release to stop).

you can change it in settings — format is like `ctrl+shift+space` or
`cmd+shift+x` on mac. takes effect after you save. note: groq also sees this as
the trigger, so if you want a different combo pick something not used by another
app on your machine.

## where settings are saved

one json file, no registry nonsense, no roaming config. the app puts it in your
config dir:

- linux: `~/.config/handy-groq/settings.json`
- macos: `~/Library/Application Support/handy-groq/settings.json`
- windows: `%APPDATA%\handy-groq\settings.json`

your api key is stored in there as plain text — that's intentional because it
stays on your machine and groq's api is the only thing that ever sees your audio.

## privacy

your mic audio only ever goes to the groq api over https. nothing is stored
locally, nothing is cached, nothing is logged to disk by the app. the api key
lives in that one settings file on your computer and nowhere else. once
transcription comes back, the audio's gone.

## builds

all releases are built on github actions (see build.yml). there's no code
signing on any platform — download the zip, extract, run `handy-groq`. on macos
you'll need to right-click-open the first time since it's unsigned.

built with: eframe 0.36, cpal, hound, reqwest + tokio, global-hotkey 0.8,
enigo 0.6, arboard 3.

---

Built in Rust. No webview. No local models. Just you, your mic, and Groq.
