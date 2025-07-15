<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { getToken } from '@/utils.js';
import * as interceptorWasm from "interceptor-wasm"
import { getCurrentInstance } from 'vue';
const { appContext } = getCurrentInstance();
const backend_url = appContext.config.globalProperties.$backend_url;

const profile = ref({
    username: "",
    bio: "",
    joined: "",
    favorites: [],
    profilePicture: ""
});

const downloadProfilePicture = () => {
    if (!profile.value.username) return;

    // Use the new download endpoint
    window.location.href = `${backend_url}/download-profile/${profile.value.username}`;
};

onMounted(() => {
    const token = getToken('jwt');
    if (!token) {
        console.error('No token found');
        return;
    }

    const payload = JSON.parse(atob(token.split('.')[1]));
    const username = payload.username;

    interceptorWasm.fetch(`${backend_url}/profile/${username}`)
        .then(response => response.json())
        .then(data => {
            profile.value.username = username;

            if (data.metadata) {
                profile.value = {
                    ...profile.value,
                    bio: data.metadata.bio || "",
                    joined: data.metadata.joined || "",
                    favorites: data.metadata.favorites || [],
                    profilePicture: data.profilePicture || ""
                };
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
            <!-- Profile picture section -->
            <div v-if="profile.profilePicture" class="profile-picture">
                <img :src="profile.profilePicture" :alt="`${profile.username}'s profile picture`" />
            </div>
            <div v-else class="profile-picture placeholder">
                <span>{{ profile.username.charAt(0).toUpperCase() }}</span>
            </div>

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

            <button
                @click="downloadProfilePicture"
                class="download-button"
                :disabled="!profile.profilePicture"
            >
                Download Profile Picture
            </button>
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

.profile-picture {
    width: 120px;
    height: 120px;
    margin: 0 auto 1.5rem;
    border-radius: 50%;
    overflow: hidden;
    position: relative;
}

.profile-picture img {
    width: 100%;
    height: 100%;
    object-fit: cover;
}

.profile-picture.placeholder {
    background-color: #4a6fa5;
    display: flex;
    align-items: center;
    justify-content: center;
    color: white;
    font-size: 3rem;
    font-weight: bold;
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

.download-button {
    display: inline-block;
    margin-top: 1.5rem;
    padding: 0.5rem 1rem;
    background-color: #4a6fa5;
    color: white;
    text-decoration: none;
    border-radius: 4px;
    border: none;
    cursor: pointer;
    transition: background-color 0.3s;
}

.download-button:hover {
    background-color: #3a5a8f;
}

.download-button:disabled {
    background-color: #cccccc;
    cursor: not-allowed;
}
</style>
