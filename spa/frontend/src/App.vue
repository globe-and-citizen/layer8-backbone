<script setup lang="ts">
import {RouterLink, RouterView} from 'vue-router'
import {isLoggedIn, logout} from "@/utils.ts";
</script>

<template>
  <div class="navbar-container">
    <nav>
      <div class="nav-content">
        <div class="nav-links">
          <RouterLink v-if="!isLoggedIn" to="/">Home</RouterLink>
          <RouterLink v-if="!isLoggedIn" to="/register">Register</RouterLink>
          <div v-if="isLoggedIn" class="logged-in-links">
            <RouterLink to="/poems">Poems</RouterLink>
            <RouterLink to="/pictures">Pictures</RouterLink>
            <RouterLink to="/profile">Profile</RouterLink>
            <RouterLink to="/upload">Upload</RouterLink>
            <a href="/" @click.prevent="logout" class="logout-link">Logout</a>
          </div>
        </div>
      </div>
    </nav>
  </div>

  <div class="spacer"></div>

  <div class="body">
    <RouterView/>
  </div>
</template>

<style scoped>
.navbar-container {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  z-index: 1000;
  background-color: #000;
  box-shadow: 0 2px 10px rgba(0, 0, 0, 0.1);
}

nav {
  width: 100%;
  max-width: 1200px;
  margin: 0 auto;
  padding: 0 1rem;
}

.nav-content {
  display: flex;
  justify-content: center;
  align-items: center;
  height: 4rem;
}

.nav-links {
  display: flex;
  justify-content: center;
  align-items: center;
  gap: 1rem;
}

nav a {
  color: white;
  text-decoration: none;
  font-weight: 500;
  transition: all 0.3s ease;
  padding: 0.5rem 1rem;
  border-radius: 4px;
}

nav a:hover {
  color: #ccc;
  background-color: rgba(255, 255, 255, 0.1);
}

nav a.router-link-exact-active {
  color: #fff;
  background-color: rgba(255, 255, 255, 0.2);
}

.logout-link {
  cursor: pointer;
}

.logout-link:hover {
  color: #ff6b6b;
}

.spacer {
  height: 4rem;
}

@media (max-width: 768px) {
  .nav-links {
    flex-wrap: wrap;
    gap: 0.5rem;
    padding: 0.5rem 0;
  }
  
  nav a {
    padding: 0.5rem;
    font-size: 0.9rem;
  }
  
  .spacer {
    height: 3.5rem;
  }
}
</style>