#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');

console.log('Running integration tests...');

const binaryName = process.platform === 'win32' ? 'template-mcp-server.exe' : 'template-mcp-server';
const binaryPath = path.join(__dirname, '..', 'dist', binaryName);

// Test 1: Initialize request
console.log('\n1. Testing initialize request...');
const init = spawn(binaryPath, [], {
  env: { ...process.env, RUST_LOG: 'error' }
});

init.stdin.write(JSON.stringify({
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {
    protocolVersion: '0.1.0',
    capabilities: {},
    clientInfo: { name: 'test-client', version: '0.1.0' }
  }
}) + '\n');

let initResponse = '';
init.stdout.on('data', (data) => {
  initResponse += data.toString();
});

init.on('close', (code) => {
  if (code !== 0 && !initResponse.includes('"result"')) {
    console.error('Initialize test failed');
    process.exit(1);
  }
  console.log('✓ Initialize request successful');
  
  // Test 2: Tools list
  testToolsList();
});

function testToolsList() {
  console.log('\n2. Testing tools/list request...');
  const tools = spawn(binaryPath, [], {
    env: { ...process.env, RUST_LOG: 'error' }
  });

  // First initialize
  tools.stdin.write(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '0.1.0',
      capabilities: {},
      clientInfo: { name: 'test-client', version: '0.1.0' }
    }
  }) + '\n');

  // Then request tools list
  setTimeout(() => {
    tools.stdin.write(JSON.stringify({
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/list',
      params: {}
    }) + '\n');
  }, 100);

  let toolsResponse = '';
  tools.stdout.on('data', (data) => {
    toolsResponse += data.toString();
  });

  setTimeout(() => {
    tools.kill();
    if (toolsResponse.includes('"tools"') && toolsResponse.includes('get_status')) {
      console.log('✓ Tools list request successful');
      testToolCall();
    } else {
      console.error('Tools list test failed');
      process.exit(1);
    }
  }, 500);
}

function testToolCall() {
  console.log('\n3. Testing tool call...');
  const call = spawn(binaryPath, [], {
    env: { ...process.env, RUST_LOG: 'error' }
  });

  // Initialize
  call.stdin.write(JSON.stringify({
    jsonrpc: '2.0',
    id: 1,
    method: 'initialize',
    params: {
      protocolVersion: '0.1.0',
      capabilities: {},
      clientInfo: { name: 'test-client', version: '0.1.0' }
    }
  }) + '\n');

  // Call echo tool
  setTimeout(() => {
    call.stdin.write(JSON.stringify({
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/call',
      params: {
        name: 'echo',
        arguments: {
          message: 'Hello, MCP!',
          prefix: 'Test'
        }
      }
    }) + '\n');
  }, 100);

  let callResponse = '';
  call.stdout.on('data', (data) => {
    callResponse += data.toString();
  });

  setTimeout(() => {
    call.kill();
    if (callResponse.includes('Test: Hello, MCP!')) {
      console.log('✓ Tool call successful');
      console.log('\n✅ All integration tests passed!');
    } else {
      console.error('Tool call test failed');
      process.exit(1);
    }
  }, 500);
}

// Handle errors
process.on('unhandledRejection', (error) => {
  console.error('Test failed:', error);
  process.exit(1);
});