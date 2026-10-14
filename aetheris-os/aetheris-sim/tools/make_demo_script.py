"""Writes demo.txt: a scripted boot -> login -> home -> app grid run for the PC simulator."""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from sim_script import run

def build(s):
    for t in (400, 1500, 2800, 4200, 5200, 6400):
        s.t = t; s.snap()
    s.t = 7100; s.snap()
    s.t = 7200
    s.typ("admin"); s.key('#')
    s.key('A'); s.key('A'); s.digits("1234"); s.key('#'); s.wait(300); s.snap()
    s.key('5'); s.wait(200); s.snap()
    for _ in range(4): s.key('6')
    s.snap()

run(build, 'demo.txt')
print("wrote demo.txt")
