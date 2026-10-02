"""Aether scene director: musical timing becomes visual choreography."""

from dataclasses import dataclass
import math


@dataclass(frozen=True)
class DirectorState:
    scene_index: int = 0
    scene_morph: float = 0.0
    world_turn: float = 0.0
    beat_pulse: float = 0.0
    phrase_progress: float = 0.0


class VisualDirector:
    PHRASE_BEATS = (13, 21, 34, 21, 13)

    def __init__(self):
        self.scene_index = 0
        self.scene_age = 0.0
        self.world_turn = 0.0
        self.beat_count = 0
        self.last_phase = 0.0
        self.beat_pulse = 0.0
        self.phrase_progress = 0.0

    def reset(self):
        self.__init__()

    def update(self, metrics, dt):
        self.scene_age += dt
        wrapped = (
            metrics.beat_confidence > 0.15
            and self.last_phase > 0.72
            and metrics.beat_phase < 0.28
        )
        if wrapped:
            self.beat_count += 1
            self.beat_pulse = 1.0
            phrase = self.PHRASE_BEATS[self.scene_index % len(self.PHRASE_BEATS)]
            if self.beat_count >= phrase:
                self.scene_index = (self.scene_index + 1) % 5
                self.beat_count = 0
                self.scene_age = 0.0

        self.last_phase = metrics.beat_phase
        self.beat_pulse *= math.exp(-dt * 7.0)
        phrase = self.PHRASE_BEATS[self.scene_index % len(self.PHRASE_BEATS)]
        self.phrase_progress = min(1.0, self.beat_count / max(1, phrase))

        target_speed = 0.10 + metrics.density * 0.22 + metrics.onset * 0.32 + metrics.high_transient * 0.18
        self.world_turn += dt * target_speed

        slow = 0.5 + 0.5 * math.sin(self.scene_age * (0.15 + self.scene_index * 0.017))
        scene_morph = min(1.0, 0.58 * slow + 0.42 * self.beat_pulse)

        return DirectorState(
            scene_index=self.scene_index,
            scene_morph=scene_morph,
            world_turn=self.world_turn,
            beat_pulse=self.beat_pulse,
            phrase_progress=self.phrase_progress,
        )
