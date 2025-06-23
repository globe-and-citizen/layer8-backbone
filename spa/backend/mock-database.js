// mock-database.js
require("dotenv").config();
const BACKEND_URL = "https://localhost:6191";

module.exports = {
  poems: [
    {
      "id": 1,
      "title": "The Red Wheelbarrow",
      "author": "WILLIAM CARLOS WILLIAMS",
      "body": "so much depends,\n upon \n a red wheel\nbarrow\nglazed with rain\nwater\nbeside the white\nchickens"
    },
    {   
      "id": 2,
      "title": "We Real Cool",
      "author": "Gwendolyn Brooks",
      "body": "We real cool. We\nLeft school. We\nLurk late. We\nStrike straight. We\nSing sin. We\nThin gin. We\nJazz June. We\nDie soon."
    },
    {
      "id": 3,
      "title": "The Road Not Taken",
      "author": "ROBERT FROST",
      "body": "Two roads diverged in a yellow wood,\nAnd sorry I could not travel both\nAnd be one traveler, long I stood\nAnd looked down one as far as I could\nTo where it bent in the undergrowth;"
    },
    {
      "id": 4,
      "title": "Sonnet 18",
      "author": "William Shakespeare",
      "body": "Shall I compare thee to a summer's day?\nThou art more lovely and more temperate:\nRough winds do shake the darling buds of May,\nAnd summer's lease hath all too short a date;"
    },
    {
      "id": 5,
      "title": "The Raven",
      "author": "Edgar Allan Poe",
      "body": "Once upon a midnight dreary, while I pondered, weak and weary,\nOver many a quaint and curious volume of forgotten lore—\nWhile I nodded, nearly napping, suddenly there came a tapping,\nAs of some one gently rapping, rapping at my chamber door."
    }
  ],
  images: [
    {
      id: 1,
      name: "small_image",
      url: `${BACKEND_URL}/images/small_image.png`,
      size: "1KB"
    },
    {
      id: 2,
      name: "medium_image",
      url: `${BACKEND_URL}/images/medium_image.jpg`,
      size: "20KB"
    },
    {
      id: 3,
      name: "large_image",
      url: `${BACKEND_URL}/images/large_image.jpg`,
      size: "230KB"
    },
    {
      id: 4,
      name: "xlarge_image",
      url: `${BACKEND_URL}/images/xlarge_image.jpg`,
      size: "505KB"
    },
    {
      id: 5,
      name: "xxlarge_image",
      url: `${BACKEND_URL}/images/xxlarge_image.jpg`,
      size: "900KB"
    }
  ],
  users: [
    {
      "username": "tester",
      "password": "$2b$10$vPCe/tNw/t2MHK/tGetY1exyvp4AhTC9w6mY5jyHHRAJrClfd1yYW", // 1234
      "metadata": {
        "bio": "Test user with pre-filled metadata",
        "joined": "2023-01-01",
        "favorites": ["The Raven", "We Real Cool"]
      }
    }
  ]
};