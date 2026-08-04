// Place this file in ~/AppData/Roaming/desktop-pet/plugins/ to activate it.
// Configure Claude Code hooks in ~/.claude/settings.json:
// {
//   "hooks": {
//     "PreToolUse":  [{ "command": "curl -s -X POST http://127.0.0.1:29513/event -d \"{\\\"type\\\":\\\"task:start\\\"}\"" }],
//     "PostToolUse": [{ "command": "curl -s -X POST http://127.0.0.1:29513/event -d \"{\\\"type\\\":\\\"task:done\\\"}\"" }]
//   }
// }
pet.onEvent('task:start', function() { pet.setState('working'); });
pet.onEvent('task:done', function() {
  pet.setState('waving');
  pet.notify('Done! ✨');
});
pet.onEvent('task:fail', function() { pet.setState('idle'); });
