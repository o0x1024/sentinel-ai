// SpaceX launch data for 2015-2025 (first 10 launches as example)
export const launchData = [
  {
    id: '5eb87ce4ffd86e000604b337',
    name: 'CRS-5',
    date_utc: '2015-01-10T09:47:00.000Z',
    success: true,
    flight_number: 18,
    details: 'First successful landing on a droneship.',
  },
  {
    id: '5eb87ce5ffd86e000604b338',
    name: 'DSCOVR',
    date_utc: '2015-02-11T23:03:00.000Z',
    success: true,
    flight_number: 19,
    details: 'Deep Space Climate Observatory.',
  },
  {
    id: '5eb87ce5ffd86e000604b339',
    name: 'ABS-3A / Eutelsat 115W B',
    date_utc: '2015-03-02T03:50:00.000Z',
    success: true,
    flight_number: 20,
    details: 'Dual satellite launch.',
  },
  {
    id: '5eb87ce5ffd86e000604b33a',
    name: 'CRS-6',
    date_utc: '2015-04-14T20:10:00.000Z',
    success: true,
    flight_number: 21,
    details: 'Failed droneship landing.',
  },
  {
    id: '5eb87ce5ffd86e000604b33b',
    name: 'TürkmenÄlem 52°E / MonacoSAT',
    date_utc: '2015-04-27T23:03:00.000Z',
    success: true,
    flight_number: 22,
    details: 'First commercial launch for Turkmenistan.',
  },
  {
    id: '5eb87ce5ffd86e000604b33c',
    name: 'CRS-7',
    date_utc: '2015-06-28T14:21:00.000Z',
    success: false,
    flight_number: 23,
    details: 'In-flight failure.',
  },
  {
    id: '5eb87ce6ffd86e000604b33d',
    name: 'OG-2 Mission 2',
    date_utc: '2015-12-22T01:29:00.000Z',
    success: true,
    flight_number: 24,
    details: 'First successful landing after launch.',
  },
  {
    id: '5eb87ce6ffd86e000604b33e',
    name: 'Jason-3',
    date_utc: '2016-01-17T15:42:00.000Z',
    success: true,
    flight_number: 25,
    details: 'Oceanography satellite.',
  },
  {
    id: '5eb87ce6ffd86e000604b33f',
    name: 'SES-9',
    date_utc: '2016-03-04T23:35:00.000Z',
    success: true,
    flight_number: 26,
    details: 'Geostationary communications satellite.',
  },
  {
    id: '5eb87ce6ffd86e000604b340',
    name: 'CRS-8',
    date_utc: '2016-04-08T20:43:00.000Z',
    success: true,
    flight_number: 27,
    details: 'First successful droneship landing.',
  },
];

// Sort by date (already chronological)
export const sortedLaunches = launchData.sort((a, b) => 
  new Date(a.date_utc).getTime() - new Date(b.date_utc).getTime()
);
