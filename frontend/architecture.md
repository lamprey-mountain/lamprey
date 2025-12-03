# architecture

- TODO: write this, refactor frontend to match plan

everything is kind of messy and wherever, i will need to clean up existing code
better

## planned

- `assets/` folder for non code resources (icons, fonts)
- `styles/` frontend styling scss
- `api/` code to interact with the rest/sync apis
- `contexts/` solidjs contexts (global state)
- `hooks/` logic that requires solidjs (local state)
- `components/` reusable components
  - `atoms/` small components (like tooltips, inputs)
  - `user_settings/` user settings
  - `room_settings/` room settings
  - `admin_settings/` admin settings
  - `media/` rendering media (like message attachments)
  - `modals/` modals (popups)
- `utils/` functions/logic not tied to solidjs/tsx
- `i18n/` translation strings
