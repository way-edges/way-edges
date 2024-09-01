#!/bin/bash

jq -cM <./defs.schema.decompress.json >./defs.schema.json
jq -cM <./config.schema.decompress.json >./config.schema.json
