# architecture

the frontend is a single page application written in solid-js + pnpm

## directory structure

```
src/
│
├── app/                    # application shell, routing, provider composition
│   ├── index.tsx           # mount point, creates root
│   ├── App.tsx             # top-level layout
│   ├── providers.tsx       # composes all global context providers
│   └── router.tsx          # route tree definition
│
├── assets/                 # static, non-code resources
│   ├── fonts/              # font files, font-face scss
│   └── images/             # logos, illustrations, raster assets
│
├── styles/                 # global scss, theme variables, resets
│
├── types/                  # shared TypeScript type definitions
│
├── i18n/                   # translation strings / locale setup
│
├── api/                    # REST and sync API clients
│   ├── mod.ts              # main API entry point (use .ts, not .tsx)
│   ├── core/               # low-level fetch, auth, request/response handling
│   ├── services/           # domain-specific API modules
│   └── util.ts
│
├── lib/                    # business logic — app-specific, domain-coupled
│   ├── permissions/        # permission calculation, role resolution
│   ├── commands/           # slash command definitions and handlers
│   ├── markdown/           # markdown parser, lexer, turndown rules
│   ├── keybinds/           # keyboard shortcut definitions
│   ├── sync/               # sync-worker, db setup, offline state
│   ├── colors.ts           # color tokens / palette constants
│   ├── emoji.ts            # emoji resolution logic
│   └── pfp.ts              # profile picture / avatar fallback logic
│
├── utils/                  # generic pure helpers — no app domain coupling
│                           # string utils, date formatting, RNG, etc.
│
├── hooks/                  # shared SolidJS hooks (createFoo, useFoo)
│
├── contexts/               # ONLY global contexts — used across the whole app
│                           # currentUser, display/theme, overlay, modals, menus
│                           # feature-specific contexts live with their features
│
├── atoms/                  # design system — reusable UI primitives
│                           # inputs, dropdowns, buttons, toggles, icons, etc.
│
├── components/
│   ├── modals/             # modal/popover components
│   ├── menus/              # context menus, dropdown menus
│   ├── features/           # domain feature modules
│   │   ├── chat/           # main chat timeline
│   │   ├── editor/         # rich text editor + plugins
│   │   ├── voice/          # voice chat panels
│   │   ├── user_settings/
│   │   │   ├── index.tsx   # settings page
│   │   │   ├── Appearance.tsx
│   │   │   ├── Chat.tsx
│   │   │   ├── Notifications.tsx
│   │   │   ├── Language.tsx
│   │   │   └── Voice.tsx
│   │   ├── channel_settings/
│   │   │   ├── index.tsx   # settings page
│   │   │   ├── Permissions.tsx
│   │   │   ├── Webhooks.tsx
│   │   │   └── ...
│   │   ├── room_settings/
│   │   │   ├── index.tsx   # settings page
│   │   │   ├── Info.tsx
│   │   │   ├── Members.tsx
│   │   │   ├── Analytics.tsx
│   │   │   ├── AuditLog.tsx
│   │   │   ├── Automod.tsx
│   │   │   ├── Webhooks.tsx
│   │   │   └── ...
│   │   ├── admin_settings/
│   │   │   ├── index.tsx   # settings page
│   │   │   ├── AuditLog.tsx
│   │   │   └── ...
│   │   └── ...             # other feature directories as needed
│   └── shared/             # cross-feature components used in multiple places
│                           # ChannelNav, RoomHeader, UserProfile, MemberList,
│                           # OverwriteDropdown, PermissionSelector, etc.
│
├── avatar/                 # icon/avatar rendering components
│                           # ChannelIcon, UserAvatar, RoomIcon
│
├── media/                  # audio/video player components
│
├── routes/                 # page-level route components
│
├── modals/                 # standalone modal components (modal popups)
│
└── menus/                  # top-level menu components
```

## notes

- create feature-specific context files in their feature directory; `contexts/` is for global contexts
- use `util/` for pure helper functions, `lib/` for business logic
- `atoms/` is kind of an ad hoc design system while `components/` is for app-specific ui
- try to use `@/` (aliased to `./src/`) for imports and avoid `../` as much as possible. using `./` is ok for logically related files in the same folder
