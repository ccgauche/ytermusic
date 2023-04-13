# YTerMusic

![index](https://user-images.githubusercontent.com/16625987/202790566-9b114019-63f9-4c4b-965d-820fd0d80a17.png)

YTerMusic is a terminal based Youtube Music Player.
It's aims to be as fast and simple as possible.

## Features

- Play your Youtube Music Playlist and Supermix.
- Memory efficient (Around 20MB of RAM while fully loaded)
- Cache all downloads and store them
- Work even without connection (If musics were already downloaded)
- Automic background download manager

## Setup

- Download the latest version from `releases`
- Give `ytermusic` authentication to your account, by copying out the headers
  1. Open the https://music.youtube.com website in your browser
  2. Open the developer tools (F12)
  3. Go to the Network tab
  4. Find the request to the `music.youtube.com` document / page
  5. Copy the `Cookie` header from the associated response request
  6. Create a file in the same directory as the binary called `headers.txt`
  7. Create an entry like this `Cookie: <cookie>`
  8. Add a valid "User Agent" below the cookie, like `User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/110.0.0.0 Safari/537.36`
- Then you can start `ytermusic`

## Linux

Install the following libraries:
```
$ sudo apt install alsa-tools libasound2-dev libdbus-1-dev pkg-config
```

## Screenshots

![Screenshot 2022-11-23 145422](https://user-images.githubusercontent.com/16625987/203564779-d3ae13f9-b262-41c0-8deb-0a486124cdca.png)
![https://user-images.githubusercontent.com/16625987/163998477-0feb87a0-dfde-4940-a5d1-09807968ec6d.png](https://user-images.githubusercontent.com/16625987/163998477-0feb87a0-dfde-4940-a5d1-09807968ec6d.png)

## Building from source

- Clone the repository
- Install rust `https://rustup.rs` nightly
- Run `cargo build --release`
- The executable is in `target/release/ytermusic.exe` or `target/release/ytermusic`

## Usage

- Use your mouse to click in lists if your terminal has mouse support
- Press `Space` to play/pause
- Press `Enter` to select a playlist or a music
- Press `f` to search
- Press `s` to shuffle
- Press `Arrow Right` or `>` to skip 5 seconds
- Press `Arrow Left` or `<` to go back 5 seconds
- Press `CTRL + Arrow Right` or `CTRL + >` to go to the next song
- Press `CTRL + Arrow Left` or `CTRL + <` to go to the previous song
- Press `+` for volume up
- Press `-` for volume down
- Press `Arrow down` to scroll down
- Press `Arrow up` to scroll up
- Press `ESC` to exit the current menu
- Press `CTRL + C` or `CTRL + D` to exit

## Features and upcoming features

- [x] Playlist selector
- [x] Error message display in the TUI
- [x] Enable connection less music playing
- [ ] Cache limit to not exceed some given disk space
- [x] A download limit to stop downloading after the queue is full
- [x] Mouse support
- [x] Search
- [ ] Custom theming

## Changelog

Alpha a0.0.11

- Added scrollable music view
- Added shuffle functionality
- Fixed some crashes while resizing the app
- Added error messages for invalid headers or cookies
- Added error messages for expired cookies

Alpha a0.0.10

- Speed up the download process
- Fix the download limit
- Fix music artists getting smashed together
- Fix the download manager not downloading all musics
- Improved stability
- Improved logs and added timings to better debug

Alpha a0.0.9:

- Progress info for downloads
- Mouse support on time bar
- Vertical volume bar
- Vertical volume bar supports mouse click
- Scroll to change volume and skip in timeline
- Improved the scrolling action
- Fixed the bug where the time bar would not update
- Debouncing the search input
- Changed the location of the cache folder to follow the XDG Base Directory Specification (By @FerrahWolfeh #20)
- More configuration options (By @ccgauche and @FerrahWolfeh)

Alpha a0.0.8

- Fixed scrolling
- Fixed audio-glitches
- Removed nightly flag use

Alpha a0.0.7

- Major changes in the API
- Fixed log file bloat issue

Alpha a0.0.6

- Fix: Fix a bug where the app would crash when trying to play a song that was not downloaded
- Fix: Improve the logger to not print the same error twice
- Improved startup time
- Fixed linux build
- Changed how task are distributed to the thread pool

Alpha a0.0.5

- Added local database cache to improve IO accesses
- Added searching for musics in the local library
- Greatly improved render performance and RAM usage
- Error management and error display in specific screen

Alpha a0.0.4

- Added menu navigation
- Added searching for musics
- Added new terminal backend

Alpha a0.0.3

- Mouse support to select playlist and music
- Download limiter
- Connection less music playing

Alpha a0.0.2

- Playlist selector
- Improved error management
- Improved TUI
- Performance upgrade
- Switch to Rustls instead of openSSL
