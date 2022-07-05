#!/bin/bash

set -euo pipefail

load_example usevars

# Arrays.

RED=(A B C)

usevars 'RED[1]=X'
declare -p RED

# Associative arrays.

usevars 'GREEN[abc]=X'

usevars GREEN
declare -p GREEN
unset GREEN

declare -A GREEN
GREEN[B]=Y
usevars 'GREEN[Xyz]=AAA'
declare -p GREEN

# Unset.

usevars RED= YELLOW=
echo "${RED:-EMPTY}"
echo "${GREEN:-EMPTY}"
