<script setup lang="ts">
import {onMounted, ref} from 'vue';

const poems = ref<any[]>([]);
const selectedPoem = ref<any | null>(null);
const showModal = ref(false);
const searchId = ref<string>('');

function openPoem(id: string) {
    fetch(`http://localhost:6191/poems?id=${id}`)
        .then(response => response.json())
        .then(data => {
            selectedPoem.value = data;
            showModal.value = true;
        })
        .catch(err => {
            console.error('Error fetching poem:', err);
        });
}

function closeModal() {
    showModal.value = false;
}

function searchPoem() {
    if (!searchId.value) {
        fetchAllPoems();
        return;
    }
    
    fetch(`http://localhost:6191/poems?id=${searchId.value}`)
        .then(response => {
            if (!response.ok) throw new Error('Poem not found');
            return response.json();
        })
        .then(data => {
            poems.value = [data]; // Display single poem
        })
        .catch(err => {
            alert(err.message);
            poems.value = [];
        });
}

function fetchAllPoems() {
    fetch('http://localhost:6191/poems')
        .then(response => response.json())
        .then(data => {
            poems.value = data;
        })
        .catch(err => {
            console.error('Error fetching poems:', err);
        });
}

onMounted(() => {
    fetchAllPoems();
});
</script>

<template>
    <div class="poem-gallery">
        <h1>📖 Poem Collection</h1>
        
        <div class="search-container">
            <input 
                v-model="searchId" 
                type="number" 
                placeholder="Enter Poem ID"
                min="1"
            />
            <button @click="searchPoem">Search</button>
            <button @click="fetchAllPoems">Show All</button>
        </div>
        
        <div class="poem-list">
            <div
                class="poem-card"
                v-for="poem in poems"
                :key="poem.id"
                @click="openPoem(poem.id)"
            >
                <h2>{{ poem.title }}</h2>
                <h3>by {{ poem.author }}</h3>
                <pre>{{ poem.body.slice(0, 150) }}...</pre>
            </div>
        </div>

        <!-- Modal for full poem -->
        <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
            <div class="modal-content">
                <h2>{{ selectedPoem.title }}</h2>
                <h3>by {{ selectedPoem.author }}</h3>
                <pre>{{ selectedPoem.body }}</pre>
                <button class="close-btn" @click="closeModal">Close</button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.search-container {
    margin: 20px 0;
    display: flex;
    gap: 10px;
    justify-content: center;
}

.search-container input {
    padding: 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
    width: 150px;
}

.search-container button {
    padding: 8px 16px;
    background-color: #007bff;
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
}

.search-container button:hover {
    background-color: #0056b3;
}

/* Keep all existing styles the same */
</style>