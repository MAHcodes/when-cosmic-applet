# when-cosmic-applet — Islamic prayer times COSMIC applet

COSMIC desktop applet that displays Islamic prayer times from the [`when`](https://github.com/MAHcodes/when) CLI.

## Requirements

- [COSMIC desktop](https://github.com/pop-os/cosmic-epoch) (Pop!_OS 24.04+ or [COSMIC nightly ISO](https://github.com/pop-os/cosmic-epoch?tab=readme-ov-file#nightly-iso))
- [`when`](https://github.com/MAHcodes/when) CLI installed and working (`when next --json` must return data)

## Installation

### From source

```
git clone https://github.com/MAHcodes/when-cosmic-applet.git
cd when-cosmic-applet
cargo build --release
sudo just install
```

## Usage

Once installed, the applet appears in the COSMIC panel. Click the icon to open the popup showing all prayer times for today, with the next prayer highlighted.

### Features

- Shows next prayer name and time in the panel
- Popup with all daily prayer times
- Live countdown to next prayer
- Current prayer period indicator
- Toggle alarm daemon from the `when` CLI (`when alarm --daemon`)
- Settings persist across sessions

### Settings

From the popup you can toggle:

| Setting | Description |
|---|---|
| Show prayer name | Display the next prayer name in the panel |
| Show prayer time | Display the next prayer time in the panel |
| Show countdown | Display live countdown instead of time |
| Show current period | Show current prayer period before the arrow |
| Alarm daemon | Start/stop the `when alarm --daemon` process |

### Configuration

Settings are saved to `~/.config/when-cosmic-applet/config.json`:

```json
{
  "show_name": true,
  "show_time": true,
  "show_countdown": false,
  "show_current_period": false
}
```

## How it works

The applet shells out to the `when` CLI for prayer time data. It parses the JSON output from `when next --json` and `when all --json`, displaying the information in the COSMIC panel and popup. Data refreshes every 2 minutes.

## Related

- [`when`](https://github.com/MAHcodes/when) — Islamic prayer times CLI & alarm

## License

MIT
