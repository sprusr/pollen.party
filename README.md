# pollen.party

Pollen forecasting powered by FMI SILAM and EAN.

## Coordinates

Latlon coordinates are stored by the code in this project in the order (lon, lat).
Coordinates are formatted and displayed to users in the order (lat, lon).
Some external crates/functions use the order (lat, lon) and this is indicated in this project's code where relevant.

## TODO

- [x] Periodically fetch data - probably ~02.00 UTC each day fetch latest model run
- [x] Pretty webpage - make it look nice
- [x] Reverse geocoding - displays name of location based on coordinates
- [ ] Geocoding - enter address/city/whatever and it gets coordinates
