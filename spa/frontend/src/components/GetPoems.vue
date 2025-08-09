<script setup lang="ts">
import { onMounted, ref } from 'vue';

const poems = ref<any[]>([]);
const selectedPoem = ref<any | null>(null);
const showModal = ref(false);
const searchId = ref<string>('');
const isLoading = ref(false);
const errorMessage = ref<string | null>(null);

function openPoem(id: string) {
    isLoading.value = true;
    fetch(`http://localhost:6192/poems?id=${id}`)
        .then(response => response.json())
        .then(data => {
            selectedPoem.value = data;
            showModal.value = true;
            errorMessage.value = null;
        })
        .catch(err => {
            console.error('Error fetching poem:', err);
            errorMessage.value = 'Failed to load poem. Please try again.';
        })
        .finally(() => {
            isLoading.value = false;
        });
}

function closeModal() {
    showModal.value = false;
    selectedPoem.value = null;
}

function searchPoem() {
    if (!searchId.value) {
        fetchAllPoems();
        return;
    }

    isLoading.value = true;
    fetch(`http://localhost:6192/poems?id=${searchId.value}`)
        .then(response => {
            if (!response.ok) throw new Error('Poem not found');
            return response.json();
        })
        .then(data => {
            poems.value = [data];
            errorMessage.value = null;
        })
        .catch(err => {
            errorMessage.value = err.message;
            poems.value = [];
        })
        .finally(() => {
            isLoading.value = false;
        });
}

function fetchAllPoems() {
    isLoading.value = true;
    fetch('http://localhost:6192/poems')
        .then(response => response.json())
        .then(data => {
            poems.value = data;
            errorMessage.value = null;
        })
        .catch(err => {
            console.error('Error fetching poems:', err);
            errorMessage.value = 'Failed to load poems. Please try again.';
        })
        .finally(() => {
            isLoading.value = false;
        });
}

onMounted(() => {
    fetchAllPoems();
});
</script>

<template>
    <div class="poem-gallery">
        <header class="header">
            <h1 class="title">📖 Poem Collection</h1>
            <p class="subtitle">Discover poetry from mock database</p>
        </header>

        <div class="search-container">
            <div class="search-box">
                <input v-model="searchId" type="number" placeholder="Enter Poem ID" min="1" class="search-input"
                    @keyup.enter="searchPoem" />
                <button @click="searchPoem" class="search-button">
                    <span v-if="!isLoading">Search</span>
                    <span v-else class="spinner"></span>
                </button>
                <button @click="fetchAllPoems" class="secondary-button">
                    Show All
                </button>
            </div>
        </div>

        <div v-if="errorMessage" class="error-message">
            {{ errorMessage }}
        </div>

        <div v-if="isLoading && poems.length === 0" class="loading-container">
            <div class="spinner large"></div>
        </div>

        <div v-else class="poem-list">
            <div class="poem-card" v-for="poem in poems" :key="poem.id" @click="openPoem(poem.id)">
                <div class="card-border"></div>
                <div class="card-content">
                    <h2 class="poem-title">{{ poem.title }}</h2>
                    <h3 class="poem-author">by {{ poem.author }}</h3>
                    <div class="poem-preview">
                        <pre>{{ poem.body.slice(0, 150) }}...</pre>
                    </div>
                    <div class="read-more">Click to read more →</div>
                </div>
            </div>
        </div>

        <!-- Modal for full poem -->
        <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
            <div class="modal-content" :class="{ loading: isLoading }">
                <button class="close-btn" @click="closeModal">
                    &times;
                </button>
                <div v-if="selectedPoem" class="poem-details">
                    <h2 class="modal-title">{{ selectedPoem.title }}</h2>
                    <h3 class="modal-author">by {{ selectedPoem.author }}</h3>
                    <div class="poem-body">
                        <pre>{{ selectedPoem.body }}</pre>
                    </div>
                </div>
                <div v-else class="loading-container">
                    <div class="spinner"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped>
/* Base styles */
:root {
    --primary-color: #5e35b1;
    --secondary-color: #3949ab;
    --accent-color: #7c4dff;
    --text-color: #333;
    --light-text: #666;
    --bg-color: #f9f5ff;
    --card-bg: #ffffff;
    --shadow: 0 4px 12px rgba(0, 0, 0, 0.1);
    --radius: 8px;
    --transition: all 0.3s ease;
}

* {
    box-sizing: border-box;
    margin: 0;
    padding: 0;
}

body {
    font-family: 'Merriweather', serif;
    background-color: var(--bg-color);
    color: var(--text-color);
    line-height: 1.6;
}

/* Header styles */
.header {
    text-align: center;
    margin-bottom: 2rem;
    padding: 2rem 1rem;
    background: linear-gradient(135deg, var(--primary-color), var(--secondary-color));
    color: white;
    border-radius: 0 0 var(--radius) var(--radius);
    box-shadow: var(--shadow);
}

.title {
    font-size: 2.5rem;
    margin-bottom: 0.5rem;
    font-weight: 700;
    color: black;
}

.subtitle {
    font-size: 1.1rem;
    opacity: 0.9;
    font-weight: 300;
    color: black;
}

/* Search container */
.search-container {
    max-width: 800px;
    margin: 0 auto 2rem;
    padding: 0 1rem;
}

.search-box {
    display: flex;
    gap: 0.5rem;
    width: 100%;
}

.search-input {
    flex: 1;
    padding: 0.75rem 1rem;
    border: 2px solid #e0e0e0;
    border-radius: var(--radius);
    font-size: 1rem;
    transition: var(--transition);
    font-family: inherit;
}

.search-input:focus {
    outline: none;
    border-color: var(--accent-color);
    box-shadow: 0 0 0 3px rgba(124, 77, 255, 0.2);
}

.search-button,
.secondary-button {
    padding: 0.75rem 1.5rem;
    border: none;
    border-radius: var(--radius);
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition: var(--transition);
    display: flex;
    align-items: center;
    justify-content: center;
}

.search-button {
    background-color: rgb(192, 192, 192);
    color: var(--primary-color);
    border: 1px solid var(--primary-color);
    border-radius: 4px;
}

.search-button:hover {
    background-color: #a4ff9f;
    transform: translateY(-1px);
}

.secondary-button {
    background-color: rgb(192, 192, 192);
    color: var(--primary-color);
    border: 1px solid var(--primary-color);
    border-radius: 4px;
}

.secondary-button:hover {
    background-color: #a4ff9f;
    transform: translateY(-1px);
}

/* Poem list */
.poem-list {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 1.5rem;
    padding: 0 1rem;
    max-width: 1200px;
    margin: 0 auto;
}

.poem-card {
    background-color: var(--card-bg);
    border-radius: var(--radius);
    overflow: hidden;
    box-shadow: var(--shadow);
    transition: var(--transition);
    cursor: pointer;
    height: 100%;
    display: flex;
    flex-direction: column;
    position: relative; /* Added for border positioning */
}

/* New border element */
.card-border {
    position: absolute;
    top: 0;
    left: 0;
    right: 0;
    height: 4px;
    background: black;
}

.poem-card:hover {
    transform: translateY(-5px);
    box-shadow: 0 8px 16px rgba(0, 0, 0, 0.15);
}

.poem-card:hover .card-border {
    height: 6px;
    background: linear-gradient(90deg, 
        #ff8a00, 
        var(--accent-color), 
        #e52e71);
}

.card-content {
    padding: 1.5rem;
    flex: 1;
    display: flex;
    flex-direction: column;
    position: relative;
    z-index: 2; /* Ensure content appears above border */
    background-color: var(--card-bg);
    border: 1px solid rgba(0, 0, 0, 0.05); /* Subtle inner border */
    border-top: none; /* Remove top border as we have the colored border */
    margin-top: 4px; /* Match border height */
    border-radius: 0 0 var(--radius) var(--radius);
}

.poem-title {
    font-size: 1.3rem;
    margin-bottom: 0.5rem;
    color: var(--primary-color);
}

.poem-author {
    font-size: 1rem;
    margin-bottom: 1rem;
    color: var(--light-text);
    font-weight: 400;
    font-style: italic;
}

.poem-preview {
    margin-bottom: 1rem;
    flex: 1;
}

.poem-preview pre {
    font-family: 'Merriweather', serif;
    white-space: pre-wrap;
    font-size: 0.95rem;
    line-height: 1.7;
    color: var(--text-color);
    overflow: hidden;
    display: -webkit-box;
    -webkit-line-clamp: 6;
    -webkit-box-orient: vertical;
}

.read-more {
    color: var(--accent-color);
    font-weight: 500;
    text-align: right;
    margin-top: auto;
    transition: var(--transition);
}

.poem-card:hover .read-more {
    transform: translateX(3px);
}

/* Modal styles */
.modal-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
    backdrop-filter: blur(4px);
    animation: fadeIn 0.3s ease;
}

.modal-content {
    background-color: white;
    border-radius: var(--radius);
    width: 90%;
    max-width: 700px;
    max-height: 90vh;
    overflow-y: auto;
    padding: 2rem;
    position: relative;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.2);
    animation: slideUp 0.4s ease;
}

.modal-content.loading {
    min-height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
}

.close-btn {
    position: absolute;
    top: 1rem;
    right: 1rem;
    background: none;
    border: none;
    font-size: 1.5rem;
    cursor: pointer;
    color: var(--light-text);
    transition: var(--transition);
    width: 2.5rem;
    height: 2.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 50%;
}

.close-btn:hover {
    color: var(--primary-color);
    background-color: #f0e9ff;
}

.modal-title {
    font-size: 1.8rem;
    margin-bottom: 0.5rem;
    color: var(--primary-color);
}

.modal-author {
    font-size: 1.2rem;
    margin-bottom: 1.5rem;
    color: var(--light-text);
    font-weight: 400;
    font-style: italic;
}

.poem-body pre {
    font-family: 'Merriweather', serif;
    white-space: pre-wrap;
    font-size: 1.05rem;
    line-height: 1.8;
    color: var(--text-color);
}

/* Loading states */
.spinner {
    width: 1.5rem;
    height: 1.5rem;
    border: 3px solid rgba(255, 255, 255, 0.3);
    border-radius: 50%;
    border-top-color: white;
    animation: spin 1s ease-in-out infinite;
}

.spinner.large {
    width: 3rem;
    height: 3rem;
    border-width: 4px;
}

.loading-container {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 2rem;
}

/* Error message */
.error-message {
    background-color: #ffebee;
    color: #c62828;
    padding: 1rem;
    border-radius: var(--radius);
    margin: 1rem auto;
    max-width: 800px;
    text-align: center;
    border-left: 4px solid #c62828;
}

/* Animations */
@keyframes fadeIn {
    from {
        opacity: 0;
    }

    to {
        opacity: 1;
    }
}

@keyframes slideUp {
    from {
        opacity: 0;
        transform: translateY(20px);
    }

    to {
        opacity: 1;
        transform: translateY(0);
    }
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}

/* Responsive adjustments */
@media (max-width: 768px) {
    .poem-list {
        grid-template-columns: 1fr;
    }

    .search-box {
        flex-direction: column;
    }

    .search-button,
    .secondary-button {
        width: 100%;
    }

    .title {
        font-size: 2rem;
    }

    .modal-content {
        width: 95%;
        padding: 1.5rem;
    }
}
</style>