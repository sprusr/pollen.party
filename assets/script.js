const geoForm = document.getElementById("geo-form");
const geoButton = document.getElementById("geo")

const originalGeoButtonValue = geoButton.value;

geoButton.addEventListener('click', (event) => {
    if (!("geolocation" in navigator)) {
        setValueForMs(event.target, 'Not supported', 1000);
        return;
    }

    event.target.value = 'Loading...';

    const latInput = document.getElementById("lat");
    const lonInput = document.getElementById("lon");

    navigator.geolocation.getCurrentPosition((position) => {
        latInput.value = position.coords.latitude.toFixed(3);
        lonInput.value = position.coords.longitude.toFixed(3);
        event.target.value = originalGeoButtonValue;
        geoForm.submit();
    }, () => {
        event.target.value = originalGeoButtonValue;
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
