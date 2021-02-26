#!/bin/bash
for f in *.model; do
  echo "@@@@@@@@@@ START $f"
  python main.py ./"$f"
  echo "@@@@@@@@@@ END $f"
done
