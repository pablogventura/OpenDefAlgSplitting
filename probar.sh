#!/bin/bash
for f in *.model; do
  python main.py ./"$f"
done
