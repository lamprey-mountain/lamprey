import type { Config } from 'tailwindcss'

export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    colors: {
      "bg1": "#0c1012",
      "bg2": "#171c1f",
      "bg3": "#22292c",
      "bg4": "#303a3d",
      "sep": "#374345",
    	"fg1": "#ffffff",
    	"fg2": "#eeeeee",
    	"fg3": "#dddddd",
    	"fg4": "#cccccc",
    	"fg5": "#aaaaaa",
    },
    fontFamily: {
      sans: ["Atkinson Hyperlegible", "Inter", "system-ui", "Avenir", "Helvetica", "Arial", "sans-serif"],
    },
    boxShadow: {
      "arst": "3px 0 0 none inset",
      "asdf": "2px 2px 8px",
      "foo": "0 8px 16px inset, 0 -8px 16px inset",
    },
    animation: {
      "popupcont": "popupcont 100ms cubic-bezier(.33,1.05,.39,.92) forwards",
      "popupbase": "popupbase 150ms cubic-bezier(.42,1.31,.52,1.09) forwards",
      "popupbg": "popupbg 120ms linear forwards",
    },
    keyframes: {
      popupcont: {
        from: { translate: "0 6px", opacity: ".5" },
        to: { translate: "0 0", opacity: "1" },
      },
      popupbase: {
        from: { scale: ".9", "box-shadow": "0 0 0 #1110" },
        to: { scale: "1", "box-shadow": "4px 4px 8px #111f" },
      },
      popupbg: {
        from: { "background-color": "#1110" },
        to: { "background-color": "#111a" },
      },
    }
  },
  plugins: [],
} satisfies Config

