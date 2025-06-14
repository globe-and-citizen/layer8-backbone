<script setup lang="ts">
import {onMounted, ref} from 'vue';
import {getToken} from '@/utils.js';

const profile = ref({
    username: "",
    bio: "",
    joined: "",
    favorites: []
});

onMounted(() => {
    const token = getToken('jwt');
    if (!token) {
        console.error('No token found');
        return;
    }

    // Extract username from token
    const payload = JSON.parse(atob(token.split('.')[1]));
    const username = payload.username;

    fetch(`http://localhost:6191/profile/${username}`)
        .then(response => response.json())
        .then(data => {
            if (data.metadata) {
                profile.value = {
                    username: username,
                    bio: data.metadata.bio || "",
                    joined: data.metadata.joined || "",
                    favorites: data.metadata.favorites || []
                };
            } else {
                profile.value.username = username;
            }
        })
        .catch(err => {
            console.error('Error fetching profile:', err);
        });
});
</script>

<template>
    <div class="profile-container">
        <div class="profile-card">
            <h1>{{ profile.username }}</h1>
            <p class="bio">{{ profile.bio }}</p>
            <div class="contact">
                <p v-if="profile.joined"><strong>📅 Joined:</strong> {{ profile.joined }}</p>
                <div v-if="profile.favorites.length">
                    <p><strong>❤️ Favorites:</strong></p>
                    <ul>
                        <li v-for="(fav, index) in profile.favorites" :key="index">{{ fav }}</li>
                    </ul>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
.profile-container {
    display: flex;
    justify-content: center;
    padding: 3rem 1rem;
    min-height: 100vh;
}

.profile-card {
    background: #fff;
    padding: 2rem;
    border-radius: 1rem;
    max-width: 500px;
    text-align: center;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
}

h1 {
    margin: 0.5rem 0 0.2rem;
    font-size: 1.8rem;
}

.bio {
    font-size: 1rem;
    color: #333;
    margin-bottom: 1.5rem;
    white-space: pre-line;
}

.contact p {
    font-size: 0.95rem;
    margin: 0.5rem 0;
}

.contact ul {
    list-style: none;
    padding: 0;
}

.contact li {
    margin: 0.3rem 0;
}
</style>