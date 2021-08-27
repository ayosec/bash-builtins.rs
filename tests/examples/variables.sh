#!/bin/bash

set -euo pipefail

load_example usevars

# Read variables.

declare -i RED=1
declare GREEN=green

declare -a BLUE
declare -A YELLOW

BLUE=(A B C)

for N in $(seq -w 100)
do
  K=$(tr 0-9 A-J <<<$N)
  YELLOW[$K]=$N
done

usevars RED GREEN
usevars BLUE MISSING
usevars YELLOW | sort

declare -n NAMEREF=GREEN
GREEN="FROM REF"
usevars NAMEREF

test "$(usevars RANDOM)" != "$(usevars RANDOM)" && echo OK
test "$(usevars LINENO)" = "LINENO = \"$LINENO\"" && echo OK

# Set variables.

usevars RED=100 CYAN=ABCD
echo R=$RED
echo C=$CYAN

if usevars 'BAD NAME=1'
then
  echo validation failed
  exit 1
fi

# Unset variables
usevars RED= YELLOW=
echo "${RED:-EMPTY}"
