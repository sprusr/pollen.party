const form = document.getElementById("form");
const geolocateButton = document.getElementById("geolocate")

const originalGeolocateButtonValue = geolocateButton.value;

geolocateButton.addEventListener('click', (event) => {
    if (!("geolocation" in navigator)) {
        setValueForMs(event.target, 'Not supported', 1000);
        return;
    }

    event.target.value = 'Loading...';

    const latInput = document.getElementById("lat");
    const lonInput = document.getElementById("lon");

    navigator.geolocation.getCurrentPosition((position) => {
        latInput.value = position.coords.latitude;
        lonInput.value = position.coords.longitude;
        event.target.value = originalGeolocateButtonValue;
        form.submit();
    }, () => {
        event.target.value = originalGeolocateButtonValue;
        setValueForMs(event.target, 'Failed to get location', 5000);
    });
})

const setValueForMs = (element, text, ms) => {
    const previousValue = element.value;
    element.value = text;
    setTimeout(() => {
        if (element.value === text) {
            // if it's still the same that we set, set it back
            element.value = previousValue;
        }
    }, ms);
}
