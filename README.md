[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/L_j-PnAY)
Goal: Apply the knowledge you've learned in new ways.

# Running the Project

<details>
  <summary>Containerize the Server and Database</summary>

  1. Build the database first
     ```bash
     docker compose -f docker/local.docker-compose.yml up db -d --build
     ```

  1. Build the server
     ```bash
     docker compose -f docker/ci.docker-compose.yml up server -d --build
     ```

  1. Run the frontend
     ```bash
     cd frontend
     python3 -m http.server 8000
     ```

     Navigate to `http://localhost:8080`

  1. Cleaning up
     ```bash
     docker compose -f docker/ci.docker-compose.yml down server -v
     docker compose -f docker/local.docker-compose.yml down db -v
     ```
</details>

# Project description
This is an open-ended project. Students can extend their BearTV project or do something new from the ground up. Project ideas must be approved by Dr. Freeman.

You must give a **formal presentation** of your project in place of a final exam. Each group will have ~12 minutes to present their work. Each member of the group must speak. You should have slides. Your presentation must include a demo of your project, although it may invlude a pre-recorded screen capture. In your presentation, you should introduce the problem that you addressed, how you addressed it, technical challenges you faced, what you learned, and next steps (if you were to continue developing it).

You may use AI LLM tools to assist with the development of your project, including code assistant tools like GitHub Copilot. If you do use any AI tools, you must describe your use during your presentation.

Unless you get specific approval otherwise, your project **must** include some component deployed on a cloud hosting service. You can use AWS, GCP, Azure, etc. These services have free tiers, and you might consider looking into tiers specifically for students.

**Graudate students enrolled in CSI-5321:** You have additional requirements. See the bottom of the README.

## Milestones
- You must present your project idea to Dr. Freeman within the first week to get it approved
- You must meet with Dr. Freeman within the first 3 weeks to give a status update and discuss roadblocks
- See the course schedule spreadhseet for specific dates

## Project Ideas
- Simulate UDP packet loss and packet corruption in BearTV in a non-deterministic way (i.e., don't just drop every Nth packet). Then, extend the application protocol to be able to detect and handle this packet loss.
- Extend the BearTV protocol to support streaming images (or video!) alongside the CC data, and visually display them on the client. This should be done in such a way that it is safely deliver*able* over *any* implementation of IPv4. The images don't have to be relevant to the caption data--you can get them randomly on the server from some image source.
- Do something hands on with a video streaming protocol such as MoQ, DASH, or HLS.
- Implement QUIC
- Develop a new congestion control algorithm and evaluate it compared to existing algorithms in a realistic setting
- Make significant contributions to a relevant open-source repository (e.g., moq-rs)
- Implement a VPN
- Implement a DNS
- Do something with route optimization
- Implement an HTTP protocol and have a simple website demo

--> These are just examples. I hope that you'll come up with a better idea to suit your own interests!

## Libraries

Depending on the project, there may be helpful libraries you find to help you out. However, there may also be libraries that do all the interesting work for you. Depending on the project, you'll need to determine what should be fair game. For example, if your project is to implement HTTP, then you shouldn't leverage an HTTP library that does it for you.

If you're unsure if a library is okay to use, just ask me.

## Languages

The core of your project should, ideally, be written in Rust. Depending on the project idea, however, I'm open to allowing the use of other languages if there's a good reason for it. For me to approve such a request, the use of a different language should enable greater learning opportunities for your group.

# Submission

## Questions
- What is your project?
    - Our project is a collaborative file sharing editor. It is a hosted file server where clients can
      connect and edit in real-time together on multiple files.
- What novel work did you do?
    - We implemented conflict free replicated data time to handle real-time edits and collision resolution based on the latest research we found.
      We integrated a custom TLS-style encryption protocol to ensure clients are provided with a measure of security.
- What did you learn?
    - We learned it is difficult to handle real-time simultaneous updates with collision resistant libraries.
      It was hard for us to integrate encryption in our frontend to portray what was integrated in our backend, but
      we managed to figure it out by using the same handshake protocol that the Rust server used, with a different
      Javascript (@noble) crypto library.
- What was challenging?
  - Challenging parts of this project included:
    - Finding a collision-resolution strategy for multiple updates at the same time
    - Integrating our custom encryption protocol with our frontend
- What AI tools did you use, and what did you use them for? What were their benefits and drawbacks?
  - We used ChatGPT and ClaudeAI for various tasks including brainstorming, creating our company logo, as well as assisting with code generation
  - We found them very useful for increasing developer efficiency by automating tedious tasks such as writing HTML or JavaScript.
  - We also used them to generate Rust code as it is a syntactically difficult language, so these tools helped us connect the gap between what we conceptually wanted to do and the specific syntax to accomplish it.
  - The increase in developer efficiency was definitely a benefit.
  - However, a drawback was that ChatGPT and Claude often make mistakes that are difficult to debug, so that takes some time, but they were still time-savers overall.
- What would you do differently next time?
  - If we were to do it all over, we would probably try to create less coupling between the frontend and backend.
  - Additionally, it would be a fun extention to add certificate verification to the TLS-style encryption protocol we made.
  - Other possible extentions we could add include having the ability to upload a starting document.

## What to submit
- Push your working code to the main branch of your team's GitHub Repository before the deadline
- Edit the README to answer the above questions
- On Teams, *each* member of the group must individually upload answers to these questions:
	- What did you (as an individual) contribute to this project?
	- What did the other members of your team contribute?
	- Do you have any concerns about your own performance or that of your team members? Any comments will remain confidential, and Dr. Freeman will try to address them in a way that preserves anonymity.
	- What feedback do you have about this course?

## Grading

Grading will be based on...
- The technical merit of the group's project
- The contribution of each individual group member
- Evidence of consistent work, as revealed during milestone meetings
- The quality of the final presentation

# 5321 Extra requirements

(For graduate students enrolled in CSI-5321)

Your project **must** have a research component to it. That is, you must set out to address an open research question using some networking concepts you've learned in this course. Subject to my approval, this project can be an extension of a project you're currently working on with your advisor.

Along with your code repository and presentation, you must submit a 4+ page paper (plus biblography), in double-column paper using the [ACM LaTeX template](https://www.acm.org/publications/proceedings-template) under the `sigconf` document class. This paper should include a brief introduction, related work, a description of your approach, experimental results, and a conclusion. Your goal should be to have a paper of high enough quality that it can be easily extended for a full submission to a research conference.

All other instructions for the project apply, including the above submission questions.

## 5321 Research ideas

- Investigate the efficacy of congestion control algorithms for QUIC. Design your own improvements for some application, and evaluate it.
- Propose improvements to, or novel applications of, the Media Over QUIC protocol
- Limitations of QUIC IP switching
- Quality adaptation for lossy data streaming of something other than traditional video
- Distributed databases, federation, or blockchain
- Protocol security flaws
- IoT, WiFi 8, or 6G
- CDN performance (e.g., content steering)
- Live video streaming with distributed super-resolution between server/client for higher quality and lower latency
- Mesh networking
- Machine learning for routing protocols

--> These are just examples. I hope that you'll come up with a better idea to suit your own interests!
