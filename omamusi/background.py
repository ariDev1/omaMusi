"""Bounded file-access workers; callbacks run only on the UI thread."""
import threading

from PySide6.QtCore import QObject, QTimer


class BackgroundTasks(QObject):
    """One worker and one replaceable pending task per operation category.

    Blocking filesystem calls cannot always be interrupted. Daemon workers
    therefore never delay window shutdown and never touch Qt objects.
    """

    def __init__(self, parent=None):
        super().__init__(parent)
        self._condition = threading.Condition()
        self._tasks = {}
        self._results = {}
        self._callbacks = {}
        self._workers = {}
        self._serial = 0
        self._closed = False
        self._timer = QTimer(self)
        self._timer.setInterval(15)
        self._timer.timeout.connect(self._poll)
        self._timer.start()

    def submit(self, category, operation, callback):
        with self._condition:
            if self._closed:
                return
            self._serial += 1
            token = self._serial
            self._callbacks[category] = (token, callback)
            self._tasks[category] = (token, operation)
            self._results.pop(category, None)
            if category not in self._workers:
                worker = threading.Thread(target=self._run, args=(category,), daemon=True,
                                          name=f"omaMusi {category}")
                self._workers[category] = worker
                worker.start()
            self._condition.notify_all()

    def invalidate(self, category):
        with self._condition:
            self._callbacks.pop(category, None)
            self._tasks.pop(category, None)
            self._results.pop(category, None)

    def _run(self, category):
        while True:
            with self._condition:
                self._condition.wait_for(lambda: self._closed or category in self._tasks)
                if self._closed:
                    return
                token, operation = self._tasks.pop(category)
            try:
                value, error = operation(), None
            except Exception as exception:
                value, error = None, str(exception)
            with self._condition:
                if self._closed:
                    return
                self._results[category] = (token, value, error)
            # Release captured paths/results before waiting for the next task.
            operation = value = None

    def _poll(self):
        with self._condition:
            results, self._results = self._results, {}
        for category, (token, value, error) in results.items():
            current = self._callbacks.get(category)
            if current is not None and token == current[0]:
                self._callbacks.pop(category)
                current[1](value, error)

    def close(self):
        self._timer.stop()
        with self._condition:
            self._closed = True
            self._tasks.clear()
            self._results.clear()
            self._callbacks.clear()
            self._condition.notify_all()
