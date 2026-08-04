// Place this file in ~/AppData/Roaming/desktop-pet/plugins/ to activate it.
pet.schedule('09:00', function() {
  pet.notify('Good morning! Time to get to work! ☀️');
  pet.setState('waving');
});
pet.schedule('17:00', function() {
  pet.notify('Time to wrap up for the day! 🌙');
  pet.setState('waving');
});
