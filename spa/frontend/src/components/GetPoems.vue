<script setup lang="ts">
import {onMounted, ref} from 'vue';
import {wasmBackend, getToken} from '@/utils.js';

const poems = ref([]);
const selectedPoem = ref<any | null>(null);
const showModal = ref(false);

function openPoem(id: string) {
  let token = getToken('jwt') || "";
  wasmBackend.get_poems(id, token)
      .then(data => {
        selectedPoem.value = {
          id: data.id || data["id"] || data.get("id"),
          title: data.title || data["title"] || data.get("title"),
          author: data.author || data["author"] || data.get("author"),
          content: data.content || data["content"] || data.get("content")
        }
        showModal.value = true;
        console.log("selectedPoem", selectedPoem.value)
      }).catch(err => {
    console.error('Error fetching profile:', err);
  })
}

function closeModal() {
  showModal.value = false;
}

onMounted(() => {
  let token = getToken('jwt') || "";

  wasmBackend.get_poems(null, token)
      .then(data => {
        let list = data.poems || data["poems"] || data.get("poems");

        for (let i = 0; i < list.length; i++) {
          let poem = {
            id: list[i].id || list[i]["id"] || list[i].get("id"),
            title: list[i].title || list[i]["title"] || list[i].get("title"),
            author: list[i].author || list[i]["author"] || list[i].get("author"),
            content: list[i].content || list[i]["content"] || list[i].get("content")
          }
          poems.value.push(poem);
        }
      }).catch(err => {
    console.error('Error fetching profile:', err);
  })
});
</script>

<template>
  <div class="poem-gallery">
    <h1>📖 Poem Collection</h1>
    <div class="poem-list">
      <div
          class="poem-card"
          v-for="poem in poems"
          :key="poem.id"
          @click="openPoem(`${poem.id}`)"
      >
        <h2>{{ poem.title }}</h2>
        <h3>by {{ poem.author }}</h3>
        <pre>{{ poem.content.slice(0, 150) }}...</pre>
      </div>
    </div>

    <!-- Modal for full poem -->
    <div v-if="showModal" class="modal-overlay" @click.self="closeModal">
      <div class="modal-content">
        <h2>{{ selectedPoem.title }}</h2>
        <h3>by {{ selectedPoem.author }}</h3>
        <pre>{{ selectedPoem.content }}</pre>
        <button class="close-btn" @click="closeModal">Close</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.poem-gallery {
  max-width: 1200px;
  margin: auto;
  padding: 2rem;
}

h1 {
  text-align: center;
  margin-bottom: 2rem;
}

.poem-list {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.poem-card {
  background: #f9f9f9;
  border-radius: 12px;
  padding: 1.5rem;
  box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
  transition: transform 0.2s;
}

.poem-card:hover {
  transform: scale(1.01);
}

.poem-card h2 {
  margin: 0 0 0.5rem 0;
  font-size: 1.3rem;
}

.poem-card h3 {
  margin: 0 0 1rem 0;
  font-weight: normal;
  color: #666;
}

.poem-card pre {
  white-space: pre-wrap;
  line-height: 1.6;
  font-family: inherit;
  color: #333;
}

/* Responsive layout */
@media (min-width: 768px) {
  .poem-list {
    flex-direction: row;
    flex-wrap: wrap;
    justify-content: center;
  }

  .poem-card {
    width: calc(33.33% - 2rem);
  }
}

.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 999;
}

.modal-content {
  background: white;
  padding: 2rem;
  border-radius: 10px;
  width: 90%;
  max-width: 600px;
  max-height: 80vh;
  overflow-y: auto;
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.2);
}

.modal-content h2 {
  margin-top: 0;
}

.close-btn {
  margin-top: 1.5rem;
  padding: 0.5rem 1rem;
  border: none;
  background: #333;
  color: white;
  cursor: pointer;
  border-radius: 5px;
}

.close-btn:hover {
  background: #555;
}

</style>
