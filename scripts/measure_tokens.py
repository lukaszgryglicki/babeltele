#!/usr/bin/env python3
"""Measure character and token compression of babeltele on the examples.

Usage: measure_tokens.py [--bin PATH] [--levels 1,2,3] [--extra-args "..."] [files...]
Requires: pip install tiktoken
"""
import argparse, subprocess, sys, glob, os
import tiktoken

ENCS = {"o200k": tiktoken.get_encoding("o200k_base"), "cl100k": tiktoken.get_encoding("cl100k_base")}

def toks(s, name):
    return len(ENCS[name].encode(s, disallowed_special=()))

def run(binary, args, data):
    p = subprocess.run([binary, *args], input=data.encode(), capture_output=True)
    if p.returncode != 0:
        sys.exit(f"{binary} {args} failed: {p.stderr.decode()}")
    return p.stdout.decode()

def main():
    ap = argparse.ArgumentParser()
    root = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
    ap.add_argument("--bin", default=os.path.join(root, "target/release/babeltele"))
    ap.add_argument("--levels", default="1,2,3")
    ap.add_argument("--extra-args", default="")
    ap.add_argument("files", nargs="*")
    a = ap.parse_args()
    files = a.files or sorted(glob.glob(os.path.join(root, "examples/*")))
    levels = [int(x) for x in a.levels.split(",") if x]
    extra = a.extra_args.split()
    tot = {}
    print(f"{'file':18s} {'lvl':>3s} {'chars':>13s} {'o200k tokens':>16s} {'cl100k tokens':>16s}")
    for f in files:
        src = open(f, encoding="utf-8").read()
        sc, so, sk = len(src), toks(src, "o200k"), toks(src, "cl100k")
        for lv in levels:
            out = run(a.bin, ["-l", str(lv), *extra], src)
            oc, oo, ok = len(out), toks(out, "o200k"), toks(out, "cl100k")
            t = tot.setdefault(lv, [0]*6)
            for i, v in enumerate((sc, oc, so, oo, sk, ok)): t[i] += v
            print(f"{os.path.basename(f):18s} {lv:3d} {oc:5d}/{sc:5d} {oc/sc:5.0%} {oo:6d}/{so:6d} {oo/so:6.0%}  {ok:6d}/{sk:6d} {ok/sk:6.0%}")
    for lv, t in sorted(tot.items()):
        print(f"{'TOTAL':18s} {lv:3d} {t[1]:5d}/{t[0]:5d} {t[1]/t[0]:5.0%} {t[3]:6d}/{t[2]:6d} {t[3]/t[2]:6.0%}  {t[5]:6d}/{t[4]:6d} {t[5]/t[4]:6.0%}")

if __name__ == "__main__":
    main()
