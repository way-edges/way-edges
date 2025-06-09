# 💻 Way-Edges

Sorry for the breaking changes, the readability of the previous configuration is just too bad...

As for the group feature, you might want to make it with the new `customize configuration path` and `ipc socket namespace` command line arguments.

## Breaking

- remove group
- flatten widget key
- remove add&rm ipc
- name -> namespace
- everything kebab-case
- workspace: animation_curve -> workspace_animation_curve
- text: custom preset update_with_interval_ms split into cmd&update_interval
- box: widgets key -> items
- box: flatten widget key

## Other changes

- (@oliviafloof ) nix: add metadata and formatter #132
- (@psi4j ) feat: Add focused_only workspace widget support, and slight change to hyprland workspace behavior #136
- ipc socket file namespace
- customize configuration path
- box: index key is optional
- bump lots of deps
