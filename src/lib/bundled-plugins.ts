export const SCHEDULE_REMINDER_CODE = `
pet.schedule('09:00', function() {
  pet.notify('Good morning! Time to get to work! ☀️');
  pet.setState('waving');
});
pet.schedule('17:00', function() {
  pet.notify('Time to wrap up for the day! 🌙');
  pet.setState('waving');
});
`;

export const CLAUDE_CODE_PROGRESS_CODE = `
pet.onEvent('task:start', function() { pet.setState('working'); });
pet.onEvent('task:done', function() {
  pet.setState('waving');
  pet.notify('Done! ✨');
});
pet.onEvent('task:fail', function() { pet.setState('idle'); });
`;
