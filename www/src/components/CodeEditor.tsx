"use client";

import React, { useState, useEffect, useRef } from 'react';
import { FileAttributes } from 'subconverter-wasm';
import * as apiClient from '@/lib/api-client';
import Editor, { Monaco } from '@monaco-editor/react';
import { editor } from 'monaco-editor';

interface CodeEditorProps {
    filePath?: string | null;
    language?: string;
    theme?: string;
    value?: string;
    readOnly?: boolean;
    options?: editor.IStandaloneEditorConstructionOptions;
    onChange?: (value: string | undefined) => void;
    onSave?: (filePath: string, content: string) => void;
}

export default function CodeEditor({
    filePath,
    language,
    theme = 'vs-dark',
    value,
    readOnly = false,
    options,
    onChange,
    onSave
}: CodeEditorProps) {
    const [internalContent, setInternalContent] = useState<string>('');
    const [editorLanguage, setEditorLanguage] = useState<string>(language || 'plaintext');
    const [loading, setLoading] = useState(false);
    const [saving, setSaving] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [fileAttributes, setFileAttributes] = useState<FileAttributes | null>(null);
    const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null);

    const isControlled = value !== undefined;
    const displayContent = isControlled ? value : internalContent;

    // Detect language from file extension if filePath is provided and language isn't
    useEffect(() => {
        if (!filePath || language) {
            setEditorLanguage(language || 'plaintext');
            return;
        }

        const extension = filePath.split('.').pop()?.toLowerCase();
        let detectedLanguage = 'plaintext';

        switch (extension) {
            case 'js':
            case 'jsx':
                detectedLanguage = 'javascript';
                break;
            case 'ts':
            case 'tsx':
                detectedLanguage = 'typescript';
                break;
            case 'json':
                detectedLanguage = 'json';
                break;
            case 'yml':
            case 'yaml':
                detectedLanguage = 'yaml';
                break;
            case 'rs':
                detectedLanguage = 'rust';
                break;
            case 'md':
                detectedLanguage = 'markdown';
                break;
            case 'html':
                detectedLanguage = 'html';
                break;
            case 'css':
                detectedLanguage = 'css';
                break;
            case 'ini':
                detectedLanguage = 'ini';
                break;
            case 'sh':
            case 'bash':
                detectedLanguage = 'shell';
                break;
            default:
                detectedLanguage = 'plaintext';
        }

        setEditorLanguage(detectedLanguage);
    }, [filePath, language]);

    // Load file content only if not controlled and filePath changes
    useEffect(() => {
        if (isControlled || !filePath) {
            // If controlled or no path, clear attributes and don't load
            setFileAttributes(null);
            if (!isControlled) setInternalContent(''); // Clear internal if no path
            setError(null);
            return;
        }

        const loadFile = async () => {
            setLoading(true);
            setError(null);
            setFileAttributes(null);
            try {
                // Get file content
                const fileContent = await apiClient.readFile(filePath);
                setInternalContent(fileContent || '');

                // Get file attributes if available
                try {
                    const attributes = await apiClient.getFileAttributes(filePath);
                    setFileAttributes(attributes);
                } catch (attrError) {
                    console.warn('Could not load file attributes:', attrError);
                } // Non-critical error
            } catch (err) {
                setError(err instanceof Error ? err.message : 'Failed to load file');
                console.error('Error loading file:', err);
                setInternalContent('# Error loading file');
            } finally {
                setLoading(false);
            }
        };

        loadFile();
    }, [filePath, isControlled]); // Rerun if filePath changes or becomes controlled/uncontrolled

    // Handle editor mount
    function handleEditorDidMount(editor: editor.IStandaloneCodeEditor, monaco: Monaco) {
        editorRef.current = editor;

        // Add keyboard shortcut for saving (Ctrl+S) only if not readOnly and filePath exists
        if (!readOnly && filePath) {
            editor.addCommand(
                monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS,
                () => handleSave()
            );
        }
    }

    // Save file content
    const handleSave = async () => {
        if (!filePath || readOnly || isControlled) return; // Don't save if no path, readOnly, or controlled

        setSaving(true);
        setError(null);
        try {
            const contentToSave = internalContent; // In uncontrolled mode, internal state is the source
            await apiClient.writeFile(filePath, contentToSave);

            // Refresh file attributes after save
            try {
                const attributes = await apiClient.getFileAttributes(filePath);
                setFileAttributes(attributes);
            } catch (attrError) {
                console.warn('Could not refresh file attributes:', attrError);
            }

            if (onSave) onSave(filePath, contentToSave);
        } catch (err) {
            setError(err instanceof Error ? err.message : 'Failed to save file');
            console.error('Error saving file:', err);
        } finally {
            setSaving(false);
        }
    };

    // Handle content change
    const handleEditorChange = (newValue: string | undefined) => {
        const currentVal = newValue || '';
        if (!isControlled) {
            setInternalContent(currentVal); // Update internal state if uncontrolled
        }
        if (onChange) {
            onChange(currentVal); // Always call onChange for parent component
        }
    };

    const finalOptions: editor.IStandaloneEditorConstructionOptions = {
        minimap: { enabled: true },
        fontSize: 14,
        wordWrap: 'on',
        scrollBeyondLastLine: false,
        automaticLayout: true,
        tabSize: 2,
        lineNumbers: 'on',
        readOnly: readOnly, // Set readOnly status
        ...options, // Merge with any additional options passed in
    };

    const showSaveButton = filePath && !readOnly && !isControlled;
    const showFileInfo = fileAttributes && !readOnly; // Show file info only if not readOnly

    return (
        <div className="h-full flex flex-col">
            {/* Header with file info and save button */}
            <div className="flex justify-between items-center p-2 border-b border-gray-700">
                <div className="flex items-center space-x-2 overflow-hidden">
                    <h3 className="text-sm font-semibold truncate text-gray-200">
                        {filePath || 'Unsaved Content'} {readOnly ? '(Read-only)' : ''}
                    </h3>
                    {showFileInfo && (
                        <div className="text-xs bg-gray-700 text-gray-200 px-2 py-0.5 rounded">
                            {apiClient.formatFileSize(fileAttributes.size)}
                        </div>
                    )}
                </div>
                {showSaveButton && (
                    <button
                        className={`px-3 py-1 rounded text-sm ${saving || loading
                            ? 'bg-gray-600 text-gray-300 cursor-not-allowed'
                            : 'bg-blue-600 hover:bg-blue-700 text-white'
                            }`}
                        onClick={handleSave}
                        disabled={saving || loading}
                    >
                        {saving ? 'Saving...' : 'Save'}
                    </button>
                )}
            </div>

            {/* Editor area */}
            <div className="flex-grow relative">
                {(loading && !isControlled) ? (
                    <div className="absolute inset-0 flex items-center justify-center">
                        <div className="flex flex-col items-center">
                            <div className="animate-spin rounded-full h-8 w-8 border-t-2 border-b-2 border-blue-500 mb-2"></div>
                            <div className="text-gray-300">Loading...</div>
                        </div>
                    </div>
                ) : error ? (
                    <div className="absolute inset-0 flex items-center justify-center text-red-400 p-4 text-center bg-gray-800">
                        {error}
                    </div>
                ) : (!filePath && !isControlled) ? (
                    <div className="absolute inset-0 flex items-center justify-center text-gray-300 p-4 text-center">
                        Select a file or provide content
                    </div>
                ) : (
                    <Editor
                        height="100%"
                        language={editorLanguage}
                        theme={theme}
                        value={displayContent}
                        onChange={handleEditorChange}
                        onMount={handleEditorDidMount}
                        options={finalOptions}
                    />
                )}
            </div>

            {/* File info footer */}
            {showFileInfo && (
                <div className="border-t border-gray-700 px-2 py-1 text-xs text-gray-300 flex justify-between">
                    <div>Type: {fileAttributes.file_type || 'Unknown'}</div>
                    <div>
                        Modified: {apiClient.formatTimestamp(Number(fileAttributes.modified_at))}
                    </div>
                </div>
            )}
        </div>
    );
} 