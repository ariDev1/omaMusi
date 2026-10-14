"""Temporary migration oracle; run from the repo with the baseline Python environment.

QT_QPA_PLATFORM=offscreen PYTHONPATH=. .venv/bin/python scripts/export-visual-reference.py
Source: development d000103; output is retained after Python retirement.
"""
import json
import math
from pathlib import Path
from dataclasses import asdict
import numpy as np
from PySide6.QtWidgets import QApplication
from omamusi.visualizer import Visualizer
from omamusi.visual.analysis import MusicMetrics, MusicAnalyzer
from omamusi.visual.director import VisualDirector
from omamusi.visual.horizon import wave_payload
from omamusi.visual.waveform import WaveformPersistence


class Clock:
    def nsecsElapsed(self):
        return 16_666_667

    def restart(self):
        pass


app = QApplication([])
visual = Visualizer()
visual.clock = Clock()
frames = []
for index in range(48):
    rate = 48000 if index < 32 else 96000
    frequency = [0, 110, 440, 4000][(index // 8) % 4]
    amplitude = [0.0, .12, .6, 1.0][(index // 8) % 4]
    pcm = np.array([amplitude * math.sin(2*math.pi*frequency*i/rate) for i in range(4096)], dtype=np.float32)
    visual.mode = 2
    visual.active = index < 44
    visual.sample_rate = rate
    if index != 45:
        visual.feed(pcm)
    visual.tick()
    fields = ["energy", "bass", "treble", "time", "phi_pulse", "phi_bloom", "phi_tension", "phi_event", "phi_velocity", "phi_impulse", "aether_onset", "aether_beat_phase", "aether_beat_confidence", "aether_density", "aether_scene_morph", "aether_world_turn", "aether_beat_pulse", "aether_phrase_progress"]
    frames.append({"index":index,"rate":rate,"frequency":frequency,"amplitude":amplitude,"active":visual.active,"fresh":index!=45,"values":{name:float(getattr(visual,name)) for name in fields},"bands":visual.bands.tolist(),"peaks":visual.peaks.tolist(),"waves":wave_payload(visual.bursts).tolist(),"spots":visual.horizon_hotspots.payload().tolist(),"history_tail":visual.history[:,-1].tolist(),"wave_density_sum":float(visual.wave_persistence.density.sum()),"wave_density_max":float(visual.wave_persistence.density.max()),"wave_alpha":visual.wave_persistence.latest_alpha})

director = VisualDirector()
directed = []
for index in range(100):
    metrics = MusicMetrics(onset=.6,beat_phase=.9 if index % 2==0 else .1,beat_confidence=.8,density=.4,high_transient=.2)
    directed.append(asdict(director.update(metrics,1/60)))

wave = WaveformPersistence()
wave.step(1/60,np.linspace(-.5,.5,1024,dtype=np.float32))
pixels = wave.rgba()
positions = [(0,0),(120,384),(118,384),(140,0),(84,767),(120,383)]
wave_probe = [{"row":r,"column":c,"density":float(wave.density[r,c]),"rgba":pixels[r,c].tolist()} for r,c in positions]
fixtures = Path("tests/rust/fixtures")
fixtures.mkdir(parents=True,exist_ok=True)
(fixtures/"visual-reference.json").write_text(json.dumps({"source":"d000103", "dt":.016666667,"frames":frames,"director":directed,"wave_probes":wave_probe},indent=2)+"\n")
analyzer = MusicAnalyzer()
beat_frames = []
for index in range(300):
    hit = index % 25 == 0
    bands = np.zeros(96,dtype=np.float32)
    bands[:18] = .8 if hit else 0
    beat_frames.append(asdict(analyzer.update(.02,bands,.4 if hit else 0,.8 if hit else 0,.1 if hit else 0)))
(fixtures/"beat-reference.json").write_text(json.dumps(beat_frames,indent=2)+"\n")
assets = Path("assets")
(assets/"star-seeds.bin").write_bytes(visual.particle_vertices.astype("<f4").tobytes())
(assets/"star-tints.bin").write_bytes(visual.tint.astype(np.uint8).tobytes())
visual.close()
print(f"Exported {len(frames)} analysis frames, {len(directed)} director frames, waveform probes and fixed star seeds")
