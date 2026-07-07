export type ScriptMetadata = {
  name: string;
  description?: string;
  homepage_url?: string; // enforce url?
  authors?: ScriptAuthor[];
  version?: string; // enforce semver?
  license?: string; // enforce spdx?

  // extra fields?
  id?: string; // if this exists when script is created, replace/update existing script with this id
  scriptName?: string; // name is human readable, this is for identifying scripts (needs better name)
}

export type ScriptAuthor = {
  name: string;
  user?: ScriptAuthorOrigin;
  url?: string;
};

export type ScriptAuthorOrigin = {
  origin_id: string;
  hostname: string;
};

