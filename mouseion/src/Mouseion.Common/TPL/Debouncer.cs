// Mouseion - Unified media manager
// Copyright (C) 2024-2025 Mouseion Contributors
// Based on Radarr (https://github.com/Radarr/Radarr)
// Copyright (C) 2010-2025 Radarr Contributors
// SPDX-License-Identifier: GPL-3.0-or-later

using System;

namespace Mouseion.Common.TPL
{
    public class Debouncer
    {
        protected readonly Action _action;
        protected readonly System.Timers.Timer _timer;

        protected volatile int _paused;
        protected volatile bool _triggered;

        public Debouncer(Action action, TimeSpan debounceDuration)
        {
            _action = action;
            _timer = new System.Timers.Timer(debounceDuration.TotalMilliseconds);
            _timer.Elapsed += timer_Elapsed;
        }

        private void timer_Elapsed(object? sender, System.Timers.ElapsedEventArgs e)
        {
            if (_paused == 0)
            {
                _triggered = false;
                _timer.Stop();
                _action();
            }
        }

        public virtual void Execute()
        {
            lock (_timer)
            {
                _triggered = true;
                if (_paused == 0)
                {
                    _timer.Start();
                }
            }
        }

        public virtual void Pause()
        {
            lock (_timer)
            {
                _paused++;
                _timer.Stop();
            }
        }

        public virtual void Resume()
        {
            lock (_timer)
            {
                _paused--;
                if (_paused == 0 && _triggered)
                {
                    _timer.Start();
                }
            }
        }
    }
}
