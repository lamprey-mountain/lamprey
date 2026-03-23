# architecture

- TODO: write this, refactor frontend to match plan

everything is kind of messy and wherever, i will need to clean up existing code
better

## planned

- `assets/` folder for non code resources (icons, fonts)
  - fonts, icons, images
- `styles/` frontend styling scss
- `api/` code to interact with the rest/sync apis
- `contexts/` solidjs contexts for state management
- `hooks/` reactive logic
- `routes/` route definitions/page components
- `components/` ui components
  - `atoms/` reusable components (aka design system - contains tooltips, inputs,
    Resizable, etc)
  - `modals/` modals/popups
  - `menus/` context menus
  - `features/`
    - `user_settings/` user settings
    - `room_settings/` room settings
    - `admin_settings/` admin settings
    - `chat/` the main chat timeline
    - `voice/` voice stuff
    - `editor/` rich text editor
    - may contain other one-off files for small features that dont need a full
      directory
- `utils/` helper functions/logic that dont belong anywhere else
- `i18n/` translation strings
