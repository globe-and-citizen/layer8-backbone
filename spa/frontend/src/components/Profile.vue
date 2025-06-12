<script setup lang="ts">
import {onMounted, ref} from 'vue';
import {wasmBackend, getToken} from '@/utils.js'; // Make sure this path is correct

const profile = ref({
  name: "",
  title: "",
  avatar: "",
  bio: "",
  email: "",
  location: "",
  website: ""
});

onMounted(() => {
  let token = getToken('jwt') || "";

  wasmBackend.get_profile(token)
      .then(data => {
        const metadata = data.metadata || data["metadata"] || data.get("metadata");
        profile.value = {
          name: metadata.username || metadata['username'] || metadata.get('username') || "",
          title: metadata.title || metadata['title'] || metadata.get('title') || "",
          avatar: metadata.avatar || metadata['avatar'] || metadata.get('avatar') || "",
          bio: metadata.bio || metadata['bio'] || metadata.get('bio') || "",
          email: metadata.email || metadata['email'] || metadata.get('email') || "",
          location: metadata.location || metadata['location'] || metadata.get('location') || "",
          website: metadata.website || metadata['website'] || metadata.get('website') || ""
        };
      }).catch(err => {
    console.error('Error fetching profile:', err);
  })
});
</script>

<template>
  <div class="profile-container">
    <div class="profile-card">
      <img class="avatar" :src="profile.avatar" :alt="profile.name"/>
      <h1>{{ profile.name }}</h1>
      <h2>{{ profile.title }}</h2>
      <p class="bio">{{ profile.bio }}</p>
      <div class="contact">
        <p><strong>📍 Location:</strong> {{ profile.location }}</p>
        <p><strong>✉️ Email:</strong> <a :href="`mailto:${profile.email}`">{{ profile.email }}</a></p>
        <p><strong>🌐 Website:</strong> <a :href="profile.website" target="_blank">{{ profile.website }}</a></p>
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

.avatar {
  width: 120px;
  height: 120px;
  border-radius: 50%;
  object-fit: cover;
  margin-bottom: 1rem;
}

h1 {
  margin: 0.5rem 0 0.2rem;
  font-size: 1.8rem;
}

h2 {
  margin: 0 0 1.5rem;
  font-size: 1.1rem;
  color: #777;
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

.contact a {
  color: #0077cc;
  text-decoration: none;
}

.contact a:hover {
  text-decoration: underline;
}
</style>
