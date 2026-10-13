#!/usr/bin/env python3
"""Manual cross-platform/physical-input acceptance matrix; this scaffold executes no network tests."""
import argparse,json
from pathlib import Path
parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('--plan',action='store_true');parser.add_argument('--evidence',type=Path);args=parser.parse_args()
cases=[
 ('RDP-3TABS','Three embedded Windows/Fedora tabs; active/inactive ownership and rapid switching'),
 ('RDP-TEXT','Unicode multiline text both directions; stale offers, disabled policy and modifier release'),
 ('RDP-PNG','64-square and 1440p PNG both directions; MIME/caps/pixel hashes'),
 ('RDP-KEYS','Print Screen, AltGr, Alt+Tab, Super, Ctrl+Alt+Del'),
 ('RDP-DISPLAY','1440p and 3K; fit/stretch/dynamic resolution/fullscreen; FPS and latency'),
 ('RDP-MOUSE','Physical touchpad, wheel, pointer-scale transforms and pane boundaries'),
 ('RDP-EXIT','Disconnect/focus loss/abnormal exit releases modifiers and cancels clipboard transfers')]
report={'status':'BLOCKED','reason':'This manual matrix has not been executed. The embedded backend exists; opt-in workspace_rdp.py verifies real authorized RDP layout restoration separately, not this full physical-input/clipboard matrix.','cases':[{'id':key,'status':'BLOCKED','requires':detail} for key,detail in cases]}
if args.evidence:
 if not args.evidence.is_dir():parser.error('Use an existing private evidence directory')
 (args.evidence/'rdp-acceptance.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(report,indent=2))
if not args.plan:raise SystemExit(78)
