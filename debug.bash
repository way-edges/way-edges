CURRENT_DIR=$(pwd)

if [[ $1 ]]; then
  export PATH=$CURRENT_DIR/target/release:$PATH
else
  export PATH=$CURRENT_DIR/target/debug:$PATH
fi

source ./way-edges-bash-complete.bash
