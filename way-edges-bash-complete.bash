_clap_complete_way_edges() {

  # combine words separated by ':'
  local cur cword words
  _get_comp_words_by_ref -n : cur cword words

  local IFS=$'\013'
  local _CLAP_COMPLETE_INDEX=${COMP_CWORD}
  local _CLAP_COMPLETE_COMP_TYPE=${COMP_TYPE}
  if compopt +o nospace 2>/dev/null; then
    local _CLAP_COMPLETE_SPACE=false
  else
    local _CLAP_COMPLETE_SPACE=true
  fi

  COMPREPLY=($(
    _CLAP_IFS="$IFS" \
      _CLAP_COMPLETE_INDEX="$cword" \
      _CLAP_COMPLETE_COMP_TYPE="$_CLAP_COMPLETE_COMP_TYPE" \
      COMPLETE="bash" \
      way-edges -- "${words[@]}"
  ))

  # remove things before ':' in COMPREPLY ?
  __ltrim_colon_completions "$cur"

  if [[ $? != 0 ]]; then
    unset COMPREPLY
  elif [[ $_CLAP_COMPLETE_SPACE == false ]] && [[ "${COMPREPLY-}" =~ [=/:]$ ]]; then
    compopt -o nospace
  fi

}
if [[ "${BASH_VERSINFO[0]}" -eq 4 && "${BASH_VERSINFO[1]}" -ge 4 || "${BASH_VERSINFO[0]}" -gt 4 ]]; then
  complete -o nospace -o bashdefault -o nosort -F _clap_complete_way_edges way-edges
else
  complete -o nospace -o bashdefault -F _clap_complete_way_edges way-edges
fi
