#!/bin/bash

load_example canpanic
canpanic
canpanic panic
canpanic

enable -d canpanic
load_example canpanic
echo after
canpanic
