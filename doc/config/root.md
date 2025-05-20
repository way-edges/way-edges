# Root

```jsonc
{
  "$schema": "./schema.json",
  "ensure_load_group": ["g1", "g2"],
  "groups": [
    {
      "name": "g1",
      "widgets": [],
    },
    {
      "name": "g2",
      "widgets": [],
    },
  ],
}
```

| Name              | Description                                            |
| ----------------- | ------------------------------------------------------ |
| ensure_load_group | Groups to be load on startup, put name inside          |
| groups            | List of groups, each contains a name and a widget list |
